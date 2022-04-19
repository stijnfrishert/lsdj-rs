//! Functionality for serializing and deserializing songs into LSDJ's block structure

mod decompress;

pub use decompress::decompress_until_eof;

use crate::sram::{
    song::{SongMemory, SongMemoryReadError},
    Name, NameFromBytesError,
};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use thiserror::Error;
use ux::u5;

/// The file system that LSDJ uses to compress songs that are currently not being edited
pub struct Filesystem {
    bytes: [u8; Self::LEN],
}

impl Filesystem {
    /// The maximal number of files that can be stored in the filesystem
    const FILES_CAPACITY: usize = 0x20;

    /// The length in bytes of a compression block
    const BLOCK_LEN: usize = 0x200;

    /// The amount of blocks available in the filesystem
    const BLOCKS_CAPACITY: usize = 0xC0;

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

    /// Retrieve the name of one of the files _without decompressing it first_.
    ///
    /// If a file is not use, its name is non-sensical. [`None`] is returned (even though
    /// memory for a name may exist).
    fn file_name(&self, index: u5) -> Option<Result<Name<8>, NameFromBytesError>> {
        if self.is_file_in_use(index) {
            let offset = u8::from(index) as usize * 8;
            Some(Name::from_bytes(&self.bytes[offset..offset + 8]))
        } else {
            None
        }
    }

    /// Retrieve the version of one of the files _without decompressing it first_.
    ///
    /// If a file is not use, its version is non-sensical. [`None`] is returned (even though
    /// memory for a version may exist).
    fn file_version(&self, index: u5) -> Option<u8> {
        if self.is_file_in_use(index) {
            let offset = 0x100 + u8::from(index) as usize;
            Some(self.bytes[offset])
        } else {
            None
        }
    }

    /// Decompress a file to its [`SongMemory`].
    ///
    /// If a file is not use, it doesn't have any compressed song data and [`None`] is returned.
    fn file_contents(&self, index: u5) -> Option<Result<SongMemory, SongMemoryReadError>> {
        let index = index.into();
        self.alloc_table()
            .iter()
            .find(|block| **block == index)
            .map(|block| {
                // Due to some weird quirk, the indices in the block alloc table start counting at 0,
                // while the first block is always used for the block meta-data (and block 1 and upward
                // are actually used for file data).
                //
                // This is weird, because the block values for the "jump to block" command in the compression
                // alsorithm *are* 1-indexed.
                //
                // Anyway, we're doing a +1 here.
                self.decompress(block + 1)
            })
    }

    /// Decompress a file starting at a specific block
    fn decompress(&self, block: u8) -> Result<SongMemory, SongMemoryReadError> {
        let mut reader = Cursor::new(&self.bytes);
        reader.seek(SeekFrom::Start(Self::block_offset(block) as u64))?;

        let mut memory = [0; SongMemory::LEN];
        let mut writer = Cursor::new(memory.as_mut_slice());

        decompress_until_eof(reader, &mut writer)?;

        assert_eq!(writer.stream_position()?, SongMemory::LEN as u64);

        SongMemory::from_reader(Cursor::new(memory))
    }

    /// What's the byte offset for a given block in the filesystem?
    fn block_offset(block: u8) -> usize {
        Self::BLOCK_LEN * block as usize
    }

    /// Return the part of block 0 that represents the block allocation table
    fn alloc_table(&self) -> &[u8] {
        &self.bytes[0x141..0x1FF]
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

/// Iterator over the [`File`]'s in a [`Filesystem`]
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

/// Reference to a single file in the [`Filesystem].
pub struct File<'a> {
    fs: &'a Filesystem,
    index: u5,
}

impl<'a> File<'a> {
    pub fn name(&self) -> Result<Name<8>, NameFromBytesError> {
        self.fs.file_name(self.index).unwrap()
    }

    pub fn version(&self) -> u8 {
        self.fs.file_version(self.index).unwrap()
    }

    pub fn decompress(&self) -> Result<SongMemory, SongMemoryReadError> {
        self.fs.file_contents(self.index).unwrap()
    }
}
