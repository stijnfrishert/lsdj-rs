//! The filesystem that LSDJ uses to store compressed songs in the [`SRam`](crate::sram::SRam)

pub mod serde;

use crate::sram::{
    lsdsng::LsdSng,
    song::{SongMemory, SongMemoryReadError},
    Name, NameFromBytesError,
};
use serde::{
    compress::{compress_block, CompressBlockError},
    decompress::decompress_block,
    End,
};
use std::{
    collections::HashMap,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    mem::replace,
    ops::Range,
};
use thiserror::Error;
use ux::u5;

const FILE_VERSIONS_RANGE: Range<usize> = 0x0100..0x0120;
const CHECK_RANGE: Range<usize> = 0x013E..0x0140;
const CHECK_VALUE: [u8; 2] = [0x6A, 0x6B];
const ACTIVE_FILE_INDEX: usize = 0x0140;
const NO_ACTIVE_FILE: u8 = 0xFF;
const ALLOC_TABLE_RANGE: Range<usize> = 0x0141..0x0200;
const UNUSED_BLOCK: u8 = 0xFF;

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

    /// Construct a new, empty filesystem
    pub fn new() -> Self {
        let mut bytes = [0; Self::LEN];

        bytes[CHECK_RANGE][0] = CHECK_VALUE[0];
        bytes[CHECK_RANGE][1] = CHECK_VALUE[1];
        bytes[ACTIVE_FILE_INDEX] = NO_ACTIVE_FILE;
        bytes[ALLOC_TABLE_RANGE].fill(UNUSED_BLOCK);

        Self { bytes }
    }

    /// Deserialize a filesystem from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FilesystemReadError>
    where
        R: Read,
    {
        let mut bytes = [0; Self::LEN];
        reader.read_exact(bytes.as_mut_slice())?;

        if bytes[CHECK_RANGE] != CHECK_VALUE {
            return Err(FilesystemReadError::InitializationCheckIncorrect);
        }

        Ok(Self { bytes })
    }

    // Serialize the filesystem to an I/O writer
    pub fn to_writer<W>(&self, mut writer: W) -> Result<(), io::Error>
    where
        W: Write,
    {
        writer.write_all(&self.bytes)
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

    /// Insert a new file into the filesystem
    pub fn insert_file(
        &mut self,
        file: u5,
        name: &Name<8>,
        version: u8,
        song: &SongMemory,
    ) -> Result<(), CompressBlockError> {
        // First, find out if we need to remove an old song from the filesystem

        // Second, compress the song into temporary blocks to figure out how many we need
        let blocks = {
            let mut reader = Cursor::new(song.as_slice());
            let mut free_blocks = self.free_blocks().peekable();
            let mut blocks = HashMap::new();

            loop {
                let mut block = [0; Self::BLOCK_LEN];
                let index = free_blocks.next().ok_or(CompressBlockError::NoBlockLeft)?;
                let end = compress_block(&mut reader, Cursor::new(block.as_mut_slice()), || {
                    free_blocks.peek().copied()
                })?;

                blocks.insert(index, block);

                if end == End::EndOfFile {
                    break;
                }
            }

            blocks
        };

        // Third, do the actual import
        self.file_name_mut(file).copy_from_slice(name.bytes());
        *self.file_version_mut(file) = version;

        for (index, block) in blocks {
            self.alloc_table_mut()[index as usize - 1] = file.into();
            self.block_mut(index).copy_from_slice(&block);
        }

        Ok(())
    }

    /// Iterate over the indices of all the free blocks
    fn free_blocks(&self) -> impl Iterator<Item = u8> + '_ {
        self.alloc_table()
            .iter()
            .enumerate()
            .filter_map(|(index, file)| {
                if *file == UNUSED_BLOCK {
                    Some(index as u8 + 1)
                } else {
                    None
                }
            })
    }

    /// Remove a file from the filesystem
    pub fn remove_file(&mut self, index: u5) -> Option<LsdSng> {
        if self.is_file_in_use(index) {
            let name = {
                let bytes = self.file_name_mut(index);
                let name = Name::from_bytes(bytes).unwrap_or_default();
                bytes.fill(0);
                name
            };

            let version = replace(self.file_version_mut(index), 0);

            let mut blocks = Vec::new();

            for block in self.file_blocks(index) {
                let bytes = self.block_mut(block);
                blocks.extend_from_slice(bytes);
                bytes.fill(0);
                self.alloc_table_mut()[(block - 1) as usize] = UNUSED_BLOCK;
            }

            Some(LsdSng::new(name, version, blocks))
        } else {
            None
        }
    }

    /// The index of the file the [`SRam`](crate::sram::SRam)'s working memory song is supposed to refer to
    pub fn active_file(&self) -> Option<u5> {
        match self.bytes[ACTIVE_FILE_INDEX] {
            NO_ACTIVE_FILE => None,
            index => Some(u5::new(index)),
        }
    }

    /// Return the number of blocks in use
    pub fn blocks_used_count(&self) -> usize {
        self.alloc_table()
            .iter()
            .filter(|block| **block != UNUSED_BLOCK)
            .count()
    }

    /// Decompress a file starting at a specific block
    fn decompress(&self, block: u8) -> Result<SongMemory, SongMemoryReadError> {
        let mut reader = Cursor::new(&self.bytes);
        reader.seek(SeekFrom::Start(Self::block_range(block).start as u64))?;

        let mut memory = [0; SongMemory::LEN];
        let mut writer = Cursor::new(memory.as_mut_slice());

        while let End::JumpToBlock(block) = decompress_block(&mut reader, &mut writer)? {
            reader.seek(SeekFrom::Start(Self::block_range(block).start as u64))?;
        }

        assert_eq!(writer.stream_position()?, SongMemory::LEN as u64);

        SongMemory::from_reader(Cursor::new(memory))
    }

    /// What's the byte range for a given block in the filesystem?
    fn block_range(block: u8) -> Range<usize> {
        let offset = Self::BLOCK_LEN * block as usize;
        offset..offset + Self::BLOCK_LEN
    }

    /// Access the bytes belonging to a specific block
    fn block(&self, block: u8) -> &[u8] {
        &self.bytes[Self::block_range(block)]
    }

    /// Access the bytes belonging to a specific block
    fn block_mut(&mut self, block: u8) -> &mut [u8] {
        &mut self.bytes[Self::block_range(block)]
    }

    /// Access the part of block 0 that represents the block allocation table
    fn alloc_table(&self) -> &[u8] {
        &self.bytes[ALLOC_TABLE_RANGE]
    }

    /// Access the part of block 0 that represents the block allocation table
    fn alloc_table_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[ALLOC_TABLE_RANGE]
    }

    /// Retrieve the bytes for a given file
    fn file_name(&self, file: u5) -> &[u8] {
        let offset = u8::from(file) as usize * 8;
        &self.bytes[offset..offset + 8]
    }

    /// Retrieve the bytes for a given file
    fn file_name_mut(&mut self, file: u5) -> &mut [u8] {
        let offset = u8::from(file) as usize * 8;
        &mut self.bytes[offset..offset + 8]
    }

    /// Retrieve the bytes for a given file
    fn file_version_mut(&mut self, file: u5) -> &mut u8 {
        let offset = u8::from(file) as usize;
        &mut self.bytes[FILE_VERSIONS_RANGE][offset]
    }

    /// Retrieve the indices of the blocks for a specific file
    fn file_blocks(&self, file: u5) -> Vec<u8> {
        let file = file.into();
        self.alloc_table()
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| {
                if *f == file {
                    Some(idx as u8 + 1)
                } else {
                    None
                }
            })
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

impl Default for Filesystem {
    fn default() -> Self {
        Self::new()
    }
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
        Name::from_bytes(self.fs.file_name(self.index))
    }

    pub fn version(&self) -> u8 {
        let offset = FILE_VERSIONS_RANGE.start + u8::from(self.index) as usize;
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

        let indices = self.fs.file_blocks(self.index);
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
    use std::str::FromStr;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let mut filesystem = {
            let mut bytes = Cursor::new(include_bytes!("../../../../test/92L_empty.sav"));
            bytes
                .seek(SeekFrom::Start(0x8000))
                .expect("Could not seek to filesystem start");
            Filesystem::from_reader(bytes).expect("could not parse filesystem")
        };

        assert_eq!(filesystem.active_file(), Some(u5::new(0)));

        assert!(filesystem.is_file_in_use(u5::new(0)));
        let file = filesystem.file(u5::new(0)).unwrap();
        assert_eq!(file.name(), Ok(Name::<8>::from_str("EMPTY").unwrap()));
        assert_eq!(file.version(), 0);

        let song = file.decompress().unwrap();
        assert_eq!(song.format_version(), 0x16);

        assert!(!filesystem.is_file_in_use(u5::new(1)));
        assert!(filesystem.file(u5::new(1)).is_none());

        filesystem.remove_file(u5::new(0));
        assert!(!filesystem.is_file_in_use(u5::new(0)));
    }

    #[test]
    fn insert() {
        let mut filesystem = Filesystem::new();

        filesystem
            .insert_file(
                u5::new(0),
                &"EMPTY".try_into().unwrap(),
                0,
                &SongMemory::from_bytes(include_bytes!("../../../../test/92L_empty.raw")).unwrap(),
            )
            .unwrap();
    }
}
