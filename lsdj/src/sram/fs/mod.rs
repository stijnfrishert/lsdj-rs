//! The filesystem that LSDJ uses to store compressed songs in the [`SRam`](crate::sram::SRam)

pub mod decompress;

use crate::sram::{
    lsdsng::LsdSng,
    song::{SongMemory, SongMemoryReadError},
    Name, NameFromBytesError,
};
use decompress::{decompress_block, End};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use thiserror::Error;
use ux::u5;

/// The file system that LSDJ uses to compress songs that are currently not being edited
pub struct Filesystem {
    bytes: [u8; Self::LEN],
}

impl Filesystem {
    /// The maximal number of files that can be stored in the filesystem
    pub const FILES_CAPACITY: usize = 0x20;

    /// The length in bytes of a compression block
    pub(crate) const BLOCK_LEN: usize = 0x200;

    /// The amount of blocks available in the filesystem
    pub const BLOCKS_CAPACITY: usize = 0xC0;

    /// The length in bytes of the entire filesystem
    const LEN: usize = Self::BLOCK_LEN * Self::BLOCKS_CAPACITY;

    /// Parse the entire filesystem from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FilesystemReadError>
    where
        R: Read,
    {
        let mut bytes = [0; Self::LEN];
        reader.read_exact(bytes.as_mut_slice())?;

        if bytes[0x13E] != 0x6A && bytes[0x13F] != 0x6B {
            return Err(FilesystemReadError::InitializationCheckIncorrect);
        }

        Ok(Self { bytes })
    }

    /// Does a file have contents?
    pub fn is_file_in_use(&self, index: u5) -> bool {
        let index = index.into();
        self.alloc_table().iter().any(|block| *block == index)
    }

    /// Retrieve a file from the filesystem
    pub fn file(&self, index: u5) -> Option<File> {
        if self.is_file_in_use(index) {
            Some(File { fs: self, index })
        } else {
            None
        }
    }

    /// Iterate over the files in the filesystem
    pub fn files(&self) -> Files {
        Files { fs: self, index: 0 }
    }

    /// The index of the file the [`SRam`](crate::sram::SRam)'s working memory song is supposed to refer to
    pub fn active_file(&self) -> Option<u5> {
        match self.bytes[0x140] {
            0xFF => None,
            index => Some(u5::new(index)),
        }
    }

    /// Return the number of blocks in use
    pub fn blocks_used_count(&self) -> usize {
        self.alloc_table()
            .iter()
            .filter(|block| **block != 0xFF)
            .count()
    }

    /// Decompress a file starting at a specific block
    fn decompress(&self, block: u8) -> Result<SongMemory, SongMemoryReadError> {
        let mut reader = Cursor::new(&self.bytes);
        reader.seek(SeekFrom::Start(Self::block_offset(block) as u64))?;

        let mut memory = [0; SongMemory::LEN];
        let mut writer = Cursor::new(memory.as_mut_slice());

        while let End::JumpToBlock(block) = decompress_block(&mut reader, &mut writer)? {
            reader.seek(SeekFrom::Start(Self::block_offset(block) as u64))?;
        }

        assert_eq!(writer.stream_position()?, SongMemory::LEN as u64);

        SongMemory::from_reader(Cursor::new(memory))
    }

    /// What's the byte offset for a given block in the filesystem?
    fn block_offset(block: u8) -> usize {
        Self::BLOCK_LEN * block as usize
    }

    /// Access the bytes belonging to a specific block
    fn block(&self, block: u8) -> &[u8] {
        let offset = Self::block_offset(block);
        &self.bytes[offset..offset + Self::BLOCK_LEN]
    }

    /// Return the part of block 0 that represents the block allocation table
    fn alloc_table(&self) -> &[u8] {
        &self.bytes[0x141..0x1FF]
    }

    /// Retrieve the indices of the blocks for a specific file
    fn file_block_indices(&self, file: u5) -> Vec<usize> {
        let file = file.into();
        self.alloc_table()
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| if *f == file { Some(idx + 1) } else { None })
            .collect()
    }
}

/// An error describing what could go wrong reading a [`Filesystem`] from I/O
#[derive(Debug, Error)]
pub enum FilesystemReadError {
    /// All correctly initialized filesystem memory has certain magic bytes set.
    /// This error is returned when that isn't the case during a read.
    #[error("The initialization check failed")]
    InitializationCheckIncorrect,

    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),
}

/// Iterator over all the [`File`]'s in a [`Filesystem`]
pub struct Files<'a> {
    fs: &'a Filesystem,
    index: u8,
}

impl<'a> Iterator for Files<'a> {
    type Item = Option<File<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.index as usize) < Filesystem::FILES_CAPACITY {
            let file = self.fs.file(u5::new(self.index));
            self.index += 1;
            Some(file)
        } else {
            None
        }
    }
}

/// Reference to a single file in the [`Filesystem`]
pub struct File<'a> {
    fs: &'a Filesystem,
    index: u5,
}

impl<'a> File<'a> {
    pub fn name(&self) -> Result<Name<8>, NameFromBytesError> {
        let offset = u8::from(self.index) as usize * 8;
        Name::from_bytes(&self.fs.bytes[offset..offset + 8])
    }

    pub fn version(&self) -> u8 {
        let offset = 0x100 + u8::from(self.index) as usize;
        self.fs.bytes[offset]
    }

    pub fn decompress(&self) -> Result<SongMemory, SongMemoryReadError> {
        let index = self.index.into();

        let first_block = self
            .fs
            .alloc_table()
            .iter()
            .enumerate()
            .find_map(|(block, file)| if *file == index { Some(block) } else { None })
            .unwrap();

        // Due to some weird quirk, the indices in the block alloc table start counting at 0,
        // while the first block is always used for the block meta-data (and block 1 and upward
        // are actually used for file data).
        //
        // This is weird, because the block values for the "jump to block" command in the compression
        // alsorithm *are* 1-indexed.
        //
        // Anyway, we're doing a +1 here.
        self.fs.decompress(first_block as u8 + 1)
    }

    pub fn lsdsng(&self) -> Result<LsdSng, FileToLsdSngError> {
        let name = self.name()?;

        let indices = self.fs.file_block_indices(self.index);
        let mut blocks = Vec::with_capacity(Filesystem::BLOCK_LEN * indices.len());
        for idx in indices {
            blocks.extend_from_slice(self.fs.block(idx as u8));
        }

        Ok(LsdSng::new(name, self.version(), blocks))
    }
}

/// An error describing what could go wrong convering a [`File`] to an [`LsdSng`]
#[derive(Debug, Error)]
pub enum FileToLsdSngError {
    /// All correctly initialized filesystem memory has certain magic bytes set.
    /// This error is returned when that isn't the case during a read.
    #[error("The initialization check failed")]
    Name(#[from] NameFromBytesError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::u5;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let filesystem = {
            let mut bytes = Cursor::new(include_bytes!("../../../../test/92L_empty.sav"));
            bytes
                .seek(SeekFrom::Start(0x8000))
                .expect("Could not seek to filesystem start");
            Filesystem::from_reader(bytes).expect("could not parse filesystem")
        };

        assert_eq!(filesystem.active_file(), Some(u5::new(0)));

        assert!(filesystem.is_file_in_use(u5::new(0)));
        let file = filesystem.file(u5::new(0)).unwrap();
        assert_eq!(
            file.name(),
            Ok(Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap())
        );
        assert_eq!(file.version(), 0);

        let song = file.decompress().unwrap();
        assert_eq!(song.format_version(), 0x16);

        assert!(!filesystem.is_file_in_use(u5::new(1)));
        assert!(filesystem.file(u5::new(1)).is_none());
    }
}
