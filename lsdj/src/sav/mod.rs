//! Anything having to do with LSDJ save files/SRAM (versus the ROM)

mod project;
pub mod serde;
pub mod song;

mod name;

pub use name::{FromBytesError, Name};
pub use project::Project;

use crate::u5;
use serde::decompress;
use song::SongMemory;
use std::io::{self, Read};
use thiserror::Error;

/// The SRAM of a full LSDJ save file
pub struct Sav {
    working_memory_song: SongMemory,
    blocks: [u8; 0x18000],
}

impl Sav {
    /// Construct a Sav from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SavReadError>
    where
        R: Read,
    {
        let mut wm = [0; SongMemory::LEN];
        reader.read_exact(wm.as_mut_slice())?;

        let mut blocks = [0; 0x18000];
        reader.read_exact(blocks.as_mut_slice())?;

        if blocks[0x13E] != 0x6A && blocks[0x13F] != 0x6B {
            return Err(SavReadError::SramCheckIncorrect);
        }

        Ok(Self {
            working_memory_song: SongMemory(wm),
            blocks,
        })
    }

    /// The song memory that's actively being edited
    pub fn working_memory_song(&self) -> &SongMemory {
        &self.working_memory_song
    }

    /// The index of the project the working memory song is referring to
    pub fn active_project(&self) -> Option<u5> {
        match self.blocks[0x140] {
            0xFF => None,
            index => Some(u5::new(index)),
        }
    }

    /// Does a project slots contain a project?
    pub fn is_project_in_use(&self, index: u8) -> bool {
        self.alloc_table().iter().any(|block| *block == index)
    }

    /// Retrieve the name of one of the projects without decompression
    ///
    /// If a project is not use, its name is non-sensical. None is returned (even though
    /// memory for this name exists)
    pub fn project_name(&self, index: u8) -> Option<Result<Name<8>, FromBytesError>> {
        if self.is_project_in_use(index) {
            let offset = index as usize * 8;
            Some(Name::from_bytes(&self.blocks[offset..offset + 8]))
        } else {
            None
        }
    }

    /// Retrieve the version of one of the projects without decompression
    ///
    /// If a project is not use, its version is non-sensical. None is returned (even though
    /// memory for this version exists)
    pub fn project_version(&self, index: u8) -> Option<u8> {
        if self.is_project_in_use(index) {
            let offset = 0x100 + index as usize;
            Some(self.blocks[offset])
        } else {
            None
        }
    }

    /// Decompress a project's song memory
    ///
    /// If a project is not use, it doesn't have any compressed song data, so None is returned
    pub fn decompress_song_memory(&self, index: u8) -> Option<Result<SongMemory, io::Error>> {
        match self.alloc_table().iter().find(|block| **block == index) {
            Some(block) => {
                // Due to some weird quirk, the indices in the block alloc table start counting at 0,
                // while the first block is always used for the block meta-data (and block 1 and upward
                // are actually used for project data). Hence, +1.
                let block = block + 1;
                Some(decompress(&self.blocks, block))
            }
            None => None,
        }
    }

    /// Return the part of block 0 that represents the block allocation table
    fn alloc_table(&self) -> &[u8] {
        &self.blocks[0x141..0x1FF]
    }
}

/// An error describing what could go wrong reading a [`Sav`] from I/O
#[derive(Debug, Error)]
pub enum SavReadError {
    /// All correctly initialized SRAM memory has certain magic bytes set.
    /// This error is returned when that isn't the case during an SRAM read.
    #[error("The SRAM initialization check failed")]
    SramCheckIncorrect,

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

        let sav = Sav::from_reader(Cursor::new(include_bytes!("../../../test/92L_empty.sav")))
            .expect("could not load sav");

        assert_eq!(sav.active_project(), Some(u5::new(0)));

        assert!(sav.is_project_in_use(0));
        assert_eq!(
            sav.project_name(0),
            Some(Ok(Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap()))
        );
        assert_eq!(sav.project_version(0), Some(0));
        sav.decompress_song_memory(0).unwrap().unwrap();

        assert!(!sav.is_project_in_use(1));
        assert_eq!(sav.project_name(1), None);
        assert_eq!(sav.project_version(1), None);
        assert!(sav.decompress_song_memory(1).is_none());
    }
}
