use std::io::{self, Read, Write};
use thiserror::Error;

/// A contiguous block of memory that represents unparsed song data
///
/// Future versions of this create might parse [`SongMemory`] into different formatted versions
/// of songs, but for now this suffices to import and export songs from [`SRam`](crate::sram).
pub struct SongMemory {
    /// The bytes that make up the song
    bytes: [u8; Self::LEN],
}

impl SongMemory {
    /// The number of bytes taken up by a single LSDJ song
    pub const LEN: usize = 0x8000;

    /// Initialize a new song, creating a buffer containing necessary verification bytes.
    pub fn new() -> Self {
        let bytes = Self::make_empty_song();
        Self { bytes }
    }

    /// Construct a new, empty song, ready for use
    ///
    /// This sets all the necessary verification bytes that LSDJ uses to check for memory corruption.
    pub fn make_empty_song() -> [u8; Self::LEN] {
        let mut bytes = [0; Self::LEN];
        bytes[0x0ff0..0x1000].fill(0xFF);
        for i in (0x1090..0x1290).step_by(16) {
            bytes[i..i + 2].fill(0x06);
        }
        bytes[0x1290..0x1690].fill(0xFF);
        let mut loops = 0;
        for i in (0x1dd0..0x1df9).step_by(6) {
            bytes[i..i + 6].copy_from_slice(&[0x57, 0x2d, 0x30 + loops, 0x57, 0x2d, 0x31 + loops]);
            loops += 2;
            if loops == 10 {
                loops += 7;
            }
        }
        bytes[0x1e78..0x1e7a].copy_from_slice(&[0x72, 0x62]);
        bytes[0x2080..0x2880].fill(0xFF);
        bytes[0x3e80..0x3e82].copy_from_slice(&[0x72, 0x62]);
        for i in (0x3eb0..0x3fb0).step_by(16) {
            bytes[i + 7] = 0x10;
            bytes[i + 8] = 0xff;
            bytes[i + 11] = 0x10;
            bytes[i + 12] = 0xff;
        }
        bytes[0x3fb4] = 0x80;
        bytes[0x3fba..0x3fbc].copy_from_slice(&[0x07, 0x02]);
        bytes[0x3fc0..0x3fc4].copy_from_slice(&[0x00, 0x20, 0x00, 0x01]);
        bytes[0x3fc6..0x3fca].fill(0xFF);
        for i in (0x6000..0x7000).step_by(16) {
            bytes[i..i + 16].copy_from_slice(&[
                0x71, 0x32, 0x33, 0x44, 0x45, 0x55, 0x66, 0x77, 0x78, 0x89, 0x99, 0xaa, 0xab, 0xbc,
                0xcd, 0xce,
            ]);
        }
        bytes[0x7000..0x7ff0].fill(0xFF);
        bytes[0x7ff0..0x7ff2].copy_from_slice(&[0x72, 0x62]);
        bytes[0x7fff] = 0x16;
        bytes
    }

    /// Deserialize [`SongMemory`] from bytes
    #[deprecated(note = "Use SongMemory::try_from(bytes: &[u8]) instead.")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        Self::try_from(bytes)
    }

    /// Deserialize [`SongMemory`] from an arbitrary I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FromReaderError>
    where
        R: Read,
    {
        let mut bytes = [0; Self::LEN];
        reader.read_exact(bytes.as_mut_slice())?;

        match Self::try_from(bytes.as_ref()) {
            Ok(v) => Ok(v),
            Err(e) => Err(FromReaderError::FromBytes(e))
        }
    }

    /// Serialize [`SongMemory`] to an arbitrary I/O writer
    pub fn to_writer<W>(&self, mut writer: W) -> Result<(), io::Error>
    where
        W: Write,
    {
        writer.write_all(&self.bytes)
    }

    /// The version of the format the song is encoded in
    pub fn format_version(&self) -> u8 {
        self.bytes[0x7FFF]
    }

    /// Access the bytes that make up the song
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Access the bytes that make up the song
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

/// Deserialize [`SongMemory`] from bytes
impl TryFrom<&[u8]> for SongMemory {
    type Error = FromBytesError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; Self::LEN] = value
            .try_into()
            .map_err(|_| FromBytesError::IncorrectSize)?;

        let check = |offset| bytes[offset] == 0x72 && bytes[offset + 1] == 0x62;

        if check(0x1E78) || check(0x3E80) || check(0x7FF0) {
            Ok(Self { bytes })
        } else {
            Err(FromBytesError::InitializationCheckIncorrect)
        }
    }
}

impl Default for SongMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that might be returned from [`SongMemory::from_bytes()`]
#[derive(Debug, Error)]
pub enum FromBytesError {
    /// The passed in number of bytes isn't correct
    #[error("The slice isn't of the correct size")]
    IncorrectSize,

    /// All correctly initialized song memory has certain bytes set for
    /// verification against memory corruption.
    ///
    /// This error is returned when that those bytes are faulty during a read.
    #[error("The initialization check failed")]
    InitializationCheckIncorrect,
}

/// Errors that might be returned from [`SongMemory::from_reader()`]
#[derive(Debug, Error)]
pub enum FromReaderError {
    /// Reading the bytes failed
    #[error("Something failed with I/O")]
    Read(#[from] io::Error),

    /// Deserialization from the read bytes failed
    #[error("Deserialiazation from the read bytes failed")]
    FromBytes(#[from] FromBytesError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let song = {
            let bytes = Cursor::new(include_bytes!("../../test/92L_empty.sav"));
            SongMemory::from_reader(bytes).expect("could not parse song")
        };

        assert_eq!(song.format_version(), 0x16);
    }

    #[test]
    fn make_empty_song() {
        let expected = *include_bytes!("../../test/92L_empty.raw");
        let actual = SongMemory::make_empty_song();
        assert_eq!(actual, expected);
    }
}
