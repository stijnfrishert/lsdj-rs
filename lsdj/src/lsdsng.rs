//! The `.lsdsng` format

use crate::{
    file::{filesystem::Filesystem, File, FileToLsdSngError},
    name::{self, Name},
    serde::{compress_block, decompress_block, CompressBlockError, End},
    song::{self, SongMemory},
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    slice,
};
use thiserror::Error;

/// A [`Name`], version and compressed [`SongMemory`]
///
/// Because [`SRam`](crate::sram) consists of multiple songs, artists often export/import them to/from a
/// format called `.lsdsng`. It's a simple "dumbed-down" version of the SRAM filesystem, containing the
/// song name and version along with compressed data for just _one_ song.
#[derive(Clone)]
pub struct LsdSng {
    /// The name of the song stored in the [`LsdSng`]
    pub name: Name<8>,

    /// The song version (increased with every save)
    pub version: u8,

    /// The blocks that make up the compressed [`SongMemory`]
    ///
    /// The `.lsdsng` format is weird in the sense that any block jumps in the decompression algorithm
    /// are to be discarded, because the blocks are just linearly copied over from the filesystem (which
    /// might have had blocks from other songs interleaved).
    blocks: Vec<u8>,
}

impl LsdSng {
    /// Create a new [`LsdSng`] from its parts
    pub(crate) fn new(name: Name<8>, version: u8, blocks: Vec<u8>) -> Self {
        Self {
            name,
            version,
            blocks,
        }
    }

    /// Create an [`LsdSng`] by compressing [`SongMemory`]
    pub fn from_song(
        name: Name<8>,
        version: u8,
        song: &SongMemory,
    ) -> Result<Self, CompressBlockError> {
        let mut blocks = Vec::new();

        let mut reader = Cursor::new(song.as_slice());

        // Loop until we've reached end-of-file
        loop {
            let mut block = [0; Filesystem::BLOCK_LEN];
            let end = compress_block(&mut reader, Cursor::new(block.as_mut_slice()), || {
                Some(blocks.len() as u8)
            })?;

            blocks.push(block);

            if end == End::EndOfFile {
                break;
            }
        }

        Ok(Self::new(
            name,
            version,
            blocks.iter().flatten().copied().collect(),
        ))
    }

    /// Read an [`LsdSng`] from an arbitrary I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FromReaderError>
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

    /// Deserialize an [`LsdSng`] from a path on disk (.lsdsng)
    pub fn from_path<P>(path: P) -> Result<Self, FromPathError>
    where
        P: AsRef<Path>,
    {
        let file = std::fs::File::open(path)?;
        Ok(Self::from_reader(file)?)
    }

    /// Serialize the [`LsdSng`] to an arbitrary I/O writer
    pub fn to_writer<W>(&self, mut writer: W) -> Result<(), io::Error>
    where
        W: Write,
    {
        writer.write_all(self.name.bytes())?;
        writer.write_all(slice::from_ref(&self.version))?;
        writer.write_all(&self.blocks)?;

        Ok(())
    }

    // Serialize the [`LsdSng`] to a path on disk (.lsdsng)
    pub fn to_path<P>(&self, path: P) -> Result<(), io::Error>
    where
        P: AsRef<Path>,
    {
        self.to_writer(std::fs::File::create(path)?)
    }
}

impl File for LsdSng {
    fn name(&self) -> Result<Name<8>, name::FromBytesError> {
        Ok(self.name.clone())
    }

    fn version(&self) -> u8 {
        self.version
    }

    fn decompress(&self) -> Result<SongMemory, song::FromReaderError> {
        let mut reader = Cursor::new(&self.blocks);
        let mut memory = [0; SongMemory::LEN];
        let mut writer = Cursor::new(memory.as_mut_slice());

        // .lsdsng's are weird in that they completely disregard the block jump values, and
        // assume that all blocks were serialized in order
        let mut block = 0;
        while decompress_block(&mut reader, &mut writer)? != End::EndOfFile {
            block += 1;
            reader.seek(SeekFrom::Start((block * Filesystem::BLOCK_LEN) as u64))?;
        }

        assert_eq!(writer.stream_position()?, SongMemory::LEN as u64);

        SongMemory::from_reader(Cursor::new(memory))
    }

    fn lsdsng(&self) -> Result<LsdSng, FileToLsdSngError> {
        Ok(self.clone())
    }
}

/// Errors that might be returned from [`LsdSng::from_reader()`]
#[derive(Debug, Error)]
pub enum FromReaderError {
    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Read(#[from] io::Error),

    /// Could not deserialize the name successfully
    #[error("Reading the name failed")]
    Name(#[from] name::FromBytesError),
}

/// Errors that might be returned from [`LsdSng::from_path()`]
#[derive(Debug, Error)]
pub enum FromPathError {
    /// Could not open the file for reading
    #[error("Could not open the file for reading")]
    FileOpen(#[from] io::Error),

    /// Deserialization from the file failed
    #[error("Reading the LsdSng from file failed")]
    Read(#[from] FromReaderError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, str::FromStr};

    #[test]
    fn empty() {
        let source = include_bytes!("../../test/92L_empty.lsdsng");
        let lsdsng = LsdSng::from_reader(Cursor::new(source)).unwrap();

        assert_eq!(lsdsng.name, Name::<8>::from_str("EMPTY").unwrap());

        assert_eq!(lsdsng.version, 0);

        let song = lsdsng.decompress().unwrap();
        assert_eq!(song.format_version(), 0x16);

        let mut dest = vec![0; source.len()];
        lsdsng.to_writer(Cursor::new(&mut dest)).unwrap();

        assert_eq!(&dest, source);
    }
}
