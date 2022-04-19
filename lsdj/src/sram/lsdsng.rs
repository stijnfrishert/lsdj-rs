//! The .lsdsng format

use crate::sram::{
    fs::{decompress::decompress_block, Filesystem},
    name::{Name, NameFromBytesError},
    song::{SongMemory, SongMemoryReadError},
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom},
    slice,
};
use thiserror::Error;

/// A song + a name and version
///
/// This is often used to export and import songs from/to [`SRam`]/save files
pub struct LsdSng {
    pub name: Name<8>,
    pub version: u8,
    pub blocks: Vec<u8>,
}

impl LsdSng {
    /// Read an LsdSng from I/O
    pub fn from_reader<R>(mut reader: R) -> Result<LsdSng, LsdsngFromReaderError>
    where
        R: Read,
    {
        let name = {
            let mut bytes = [0; 8];
            reader.read_exact(&mut bytes)?;
            Name::from_bytes(bytes.as_mut_slice())?
        };

        let mut version = 0;
        reader.read_exact(slice::from_mut(&mut version))?;

        let mut blocks = Vec::new();
        reader.read_to_end(&mut blocks)?;

        Ok(LsdSng {
            name,
            version,
            blocks,
        })
    }

    pub fn decompress(&self) -> Result<SongMemory, SongMemoryReadError> {
        let mut reader = Cursor::new(&self.blocks);
        let mut memory = [0; SongMemory::LEN];
        let mut writer = Cursor::new(memory.as_mut_slice());

        // .lsdsng's are weird in that they completely disregard the block jump values, and
        // assume that all blocks were serialized in order
        let mut block = 0;
        while decompress_block(&mut reader, &mut writer)?.is_some() {
            block += 1;
            reader.seek(SeekFrom::Start((block * Filesystem::BLOCK_LEN) as u64))?;
        }

        assert_eq!(writer.stream_position()?, SongMemory::LEN as u64);

        SongMemory::from_reader(Cursor::new(memory))
    }
}

/// An error describing what could go wrong reading an [`LsdSng`] from I/O
#[derive(Debug, Error)]
pub enum LsdsngFromReaderError {
    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),

    /// Could not read the name successfully
    #[error("Reading the name failed")]
    Name(#[from] NameFromBytesError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        use std::io::Cursor;

        let lsdsng =
            LsdSng::from_reader(Cursor::new(include_bytes!("../../../test/empty.lsdprj"))).unwrap();

        assert_eq!(
            lsdsng.name,
            Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap()
        );

        assert_eq!(lsdsng.version, 0);

        let song = lsdsng.decompress().unwrap();
        assert_eq!(song.format_version(), 0x16);
    }
}
