//! Song data and everything they're made of

pub mod instrument;
pub mod wave;

use std::io::{self, Read, Write};
use thiserror::Error;

/// A contiguous block of memory that represents unparsed song data
pub struct SongMemory {
    bytes: [u8; Self::LEN],
}

impl SongMemory {
    /// The number of bytes taken up by a single LSDJ song
    pub const LEN: usize = 0x8000;

    /// Construct a new, empty song, ready for use
    pub fn new() -> Self {
        Self {
            bytes: *include_bytes!("empty.raw"),
        }
    }

    /// Deserialize song memory from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SongMemoryReadError>
    where
        R: Read,
    {
        let mut bytes = [0; Self::LEN];
        reader.read_exact(bytes.as_mut_slice())?;

        let check = |offset| bytes[offset] == 0x72 && bytes[offset + 1] == 0x62;

        if check(0x1E78) || check(0x3E80) || check(0x7FF0) {
            Ok(Self { bytes })
        } else {
            Err(SongMemoryReadError::InitializationCheckIncorrect)
        }
    }

    /// Serialize song memory to an I/O writer
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
}

impl Default for SongMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// An error describing what could go wrong reading a [`SongMemory`] from I/O
#[derive(Debug, Error)]
pub enum SongMemoryReadError {
    /// All correctly initialized song memory has certain magic bytes set.
    /// This error is returned when that isn't the case during a read.
    #[error("The initialization check failed")]
    InitializationCheckIncorrect,

    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let song = {
            let bytes = Cursor::new(include_bytes!("../../../../test/92L_empty.sav"));
            SongMemory::from_reader(bytes).expect("could not parse song")
        };

        assert_eq!(song.format_version(), 0x16);
    }
}
