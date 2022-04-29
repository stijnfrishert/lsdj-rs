use super::{File, FileToLsdSngError};
use crate::{
    lsdsng::LsdSng,
    name::{FromBytesError, Name},
    serde::{compress_block, decompress_block, CompressBlockError, End},
    song::{self, SongMemory},
};
use std::{
    collections::HashMap,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    mem::replace,
    ops::Range,
};
use thiserror::Error;

/// A 5-bit (0 - 32) index into the [`Filesystem`]
pub type Index = ux::u5;

const FILE_VERSIONS_RANGE: Range<usize> = 0x0100..0x0120;
const CHECK_RANGE: Range<usize> = 0x013E..0x0140;
const CHECK_VALUE: [u8; 2] = [0x6A, 0x6B];
const ACTIVE_FILE_INDEX: usize = 0x0140;
const NO_ACTIVE_FILE: u8 = 0xFF;
const ALLOC_TABLE_RANGE: Range<usize> = 0x0141..0x0200;
const UNUSED_BLOCK: u8 = 0xFF;

/// A filesystem for storing compressed [`File`]'s
///
/// LSDJ [`SRam`](crate::sram) consists of one uncompressed song, and a filesystem storage where songs not
/// currently being worked on can be compressed and stored. [`Filesystem`] presents an interface
/// for retrieving and saving files into the storage. The actual compression algorithm is
/// implemented in [`serde`](crate::serde).
///
/// The LSDJ filesystem has a maximum capacity of 32 files, no matter how much space they take
/// up. There is space allocated for the name and version number of each file regardless. Whether
/// a file entry slot is actually in use solely depends on whether any compressed data blocks can
/// be found for that file index.
///
/// The compression itself is done in blocks of 512 bytes each, according to the specified
/// [algorithm](https://littlesounddj.fandom.com/wiki/File_Management_Structure).
pub struct Filesystem {
    bytes: [u8; Self::LEN],
}

impl Filesystem {
    /// The maximal number of files that can be stored in the filesystem
    pub const FILES_CAPACITY: usize = 0x20;

    /// The amount of blocks available in the filesystem
    pub const BLOCKS_CAPACITY: usize = 0xC0;

    /// The length in bytes of a compression block
    pub(crate) const BLOCK_LEN: usize = 0x200;

    /// The length in bytes of the entire filesystem
    const LEN: usize = Self::BLOCK_LEN * Self::BLOCKS_CAPACITY;

    /// Construct a valid, but empty filesystem
    ///
    /// When LSDJ initializes SRAM, it sets some bytes for later verification against
    /// memory corruption. This function does so too, resulting in an empty, but valid
    /// filesystem
    pub fn new() -> Self {
        let mut bytes = [0; Self::LEN];

        bytes[CHECK_RANGE][0] = CHECK_VALUE[0];
        bytes[CHECK_RANGE][1] = CHECK_VALUE[1];
        bytes[ACTIVE_FILE_INDEX] = NO_ACTIVE_FILE;
        bytes[ALLOC_TABLE_RANGE].fill(UNUSED_BLOCK);

        Self { bytes }
    }

    /// Deserialize a [`Filesystem`] from an arbitrary I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FromReaderError>
    where
        R: Read,
    {
        let mut bytes = [0; Self::LEN];
        reader.read_exact(bytes.as_mut_slice())?;

        if bytes[CHECK_RANGE] != CHECK_VALUE {
            return Err(FromReaderError::InitializationCheckIncorrect);
        }

        Ok(Self { bytes })
    }

    // Serialize the [`Filesystem`] to an arbitrary I/O writer
    pub fn to_writer<W>(&self, mut writer: W) -> Result<(), io::Error>
    where
        W: Write,
    {
        writer.write_all(&self.bytes)
    }

    /// Is any compessed song data stored for the file slot at this index?
    fn is_file_in_use(&self, index: Index) -> bool {
        let index = index.into();
        self.alloc_table().iter().any(|block| *block == index)
    }

    /// Retrieve a [`File`] [`Entry`] from the filesystem
    ///
    /// This function either returns an actual file entry in the filesystem if it
    /// can find compressed song data for the index, or [`None`] if the file slot
    /// is empty.
    ///
    /// The resulting [`Entry`] can be queried for [`Name`], version and [`SongMemory`].
    pub fn file(&self, index: Index) -> Option<Entry> {
        if self.is_file_in_use(index) {
            Some(Entry { fs: self, index })
        } else {
            None
        }
    }

    /// Iterate over all the [`File`]'s in the filesystem
    pub fn files(&self) -> Entries {
        Entries { fs: self, index: 0 }
    }

    /// Insert a new file into the filesystem
    ///
    /// This function tries to compress the provided song memory into the filesystem. It can
    /// fail if there is not enough space for the resulting compression blocks, at which point
    /// it won't insert anything at all.
    ///
    /// If a file already existed at this index, the old file is returned as an [`LsdSng`].
    pub fn insert_file(
        &mut self,
        file: Index,
        name: &Name<8>,
        version: u8,
        song: &SongMemory,
    ) -> Result<Option<LsdSng>, CompressBlockError> {
        // First, compress the song into temporary blocks to figure out how many we need
        let blocks = {
            // Figure out which blocks we *can* use
            let mut free_blocks = self
                .alloc_table()
                .iter()
                .enumerate()
                .filter_map(|(index, f)| {
                    if *f == UNUSED_BLOCK || *f == file.into() {
                        Some(index as u8 + 1)
                    } else {
                        None
                    }
                })
                .peekable();

            // Create a reader over the song memory and a hashmap to store the blocks
            let mut reader = Cursor::new(song.as_slice());
            let mut blocks = HashMap::new();

            // Loop until we've reached end-of-file
            // If we run out of space, compress_block() will return an error and this will propagate upward
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

        // Second, remove the old file if necessary
        let old = self.remove_file(file);

        // Third, do the actual import
        self.file_name_mut(file).copy_from_slice(name.bytes());
        *self.file_version_mut(file) = version;

        for (index, block) in blocks {
            self.alloc_table_mut()[index as usize - 1] = file.into();
            self.block_mut(index).copy_from_slice(&block);
        }

        Ok(old)
    }

    /// Remove a file from the filesystem
    ///
    /// Returns either the file, or [`None`] if no file at that index existed
    pub fn remove_file(&mut self, index: Index) -> Option<LsdSng> {
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

    /// The index of the file currently being worked on
    ///
    /// LSDJ's [`SRam`](crate::sram) has working memory for one uncompressed song. Usually this song represents
    /// an actively edited verson of one of the files in the filesystem.
    pub fn active_file(&self) -> Option<Index> {
        match self.bytes[ACTIVE_FILE_INDEX] {
            NO_ACTIVE_FILE => None,
            index => Some(Index::new(index)),
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
    fn decompress(&self, block: u8) -> Result<SongMemory, song::FromReaderError> {
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
    fn file_name(&self, file: Index) -> &[u8] {
        let offset = u8::from(file) as usize * 8;
        &self.bytes[offset..offset + 8]
    }

    /// Retrieve the bytes for a given file
    fn file_name_mut(&mut self, file: Index) -> &mut [u8] {
        let offset = u8::from(file) as usize * 8;
        &mut self.bytes[offset..offset + 8]
    }

    /// Retrieve the bytes for a given file
    fn file_version_mut(&mut self, file: Index) -> &mut u8 {
        let offset = u8::from(file) as usize;
        &mut self.bytes[FILE_VERSIONS_RANGE][offset]
    }

    /// Retrieve the indices of the blocks for a specific file
    fn file_blocks(&self, file: Index) -> Vec<u8> {
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

/// Errors that might occur deserializing a [`Filesystem`] from I/O
#[derive(Debug, Error)]
pub enum FromReaderError {
    /// All correctly initialized filesystem memory has certain bytes set for
    /// verification against memory corruption.
    ///
    /// This error is returned when that those bytes are faulty during a read.
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

/// Iterator over all the file [`Entry`]'s in a [`Filesystem`]
pub struct Entries<'a> {
    fs: &'a Filesystem,
    index: u8,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Option<Entry<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.index as usize) < Filesystem::FILES_CAPACITY {
            let file = self.fs.file(Index::new(self.index));
            self.index += 1;
            Some(file)
        } else {
            None
        }
    }
}

/// Immutable reference to a single [`File`] in the [`Filesystem`]
pub struct Entry<'a> {
    fs: &'a Filesystem,
    index: Index,
}

impl<'a> File for Entry<'a> {
    fn name(&self) -> Result<Name<8>, FromBytesError> {
        Name::from_bytes(self.fs.file_name(self.index))
    }

    fn version(&self) -> u8 {
        let offset = FILE_VERSIONS_RANGE.start + u8::from(self.index) as usize;
        self.fs.bytes[offset]
    }

    fn decompress(&self) -> Result<SongMemory, song::FromReaderError> {
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

    fn lsdsng(&self) -> Result<LsdSng, FileToLsdSngError> {
        let name = self.name()?;

        let indices = self.fs.file_blocks(self.index);
        let mut blocks = Vec::with_capacity(Filesystem::BLOCK_LEN * indices.len());
        for idx in indices {
            blocks.extend_from_slice(self.fs.block(idx as u8));
        }

        Ok(LsdSng::new(name, self.version(), blocks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let mut filesystem = {
            let mut bytes = Cursor::new(include_bytes!("../../test/92L_empty.sav"));
            bytes
                .seek(SeekFrom::Start(0x8000))
                .expect("Could not seek to filesystem start");
            Filesystem::from_reader(bytes).expect("could not parse filesystem")
        };

        assert_eq!(filesystem.active_file(), Some(Index::new(0)));

        assert!(filesystem.is_file_in_use(Index::new(0)));
        let file = filesystem.file(Index::new(0)).unwrap();
        assert_eq!(file.name(), Ok("EMPTY".try_into().unwrap()));
        assert_eq!(file.version(), 0);

        let song = file.decompress().unwrap();
        assert_eq!(song.format_version(), 0x16);

        assert!(!filesystem.is_file_in_use(Index::new(1)));
        assert!(filesystem.file(Index::new(1)).is_none());

        filesystem.remove_file(Index::new(0));
        assert!(!filesystem.is_file_in_use(Index::new(0)));
    }

    #[test]
    fn insert() {
        let mut filesystem = Filesystem::new();

        let name = "EMPTY".try_into().unwrap();
        let song = SongMemory::new();

        let old = filesystem
            .insert_file(Index::new(0), &name, 0, &song)
            .unwrap();

        assert!(filesystem.is_file_in_use(Index::new(0)));
        assert!(old.is_none());

        let old = filesystem
            .insert_file(Index::new(0), &name, 0, &song)
            .unwrap();
        assert!(filesystem.is_file_in_use(Index::new(0)));
        assert!(old.is_some());
    }
}
