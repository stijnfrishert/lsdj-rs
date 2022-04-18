//! Anything having to do with LSDJ SRAM/save files (versus the ROM)

mod project;
pub mod serde;
pub mod song;

mod name;

pub use name::{Name, NameFromBytesError};
pub use project::Project;

use crate::u5;
use serde::decompress;
use song::SongMemory;
use std::io::{self, Read};
use thiserror::Error;

/// The SRAM for a full LSDJ save
pub struct SRam {
    working_memory_song: SongMemory,
    blocks: [u8; 0x18000],
}

impl SRam {
    /// Parse SRAM from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SRamReadError>
    where
        R: Read,
    {
        let mut wm = [0; SongMemory::LEN];
        reader.read_exact(wm.as_mut_slice())?;

        let mut blocks = [0; 0x18000];
        reader.read_exact(blocks.as_mut_slice())?;

        if blocks[0x13E] != 0x6A && blocks[0x13F] != 0x6B {
            return Err(SRamReadError::CheckIncorrect);
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
    pub fn is_project_in_use(&self, index: u5) -> bool {
        let index = index.into();
        self.alloc_table().iter().any(|block| *block == index)
    }

    /// Retrieve a full project: its name, version and _decompressing its song data_.
    pub fn project(&self, index: u5) -> Option<Result<Project, ProjectRetrieveError>> {
        let song = self.decompress_song_memory(index)?;
        let song = match song {
            Ok(song) => song,
            Err(err) => return Some(Err(ProjectRetrieveError::Decompression(err))),
        };

        let name = match self
            .project_name(index)
            .expect("A name should be available, because a song is")
        {
            Ok(name) => name,
            Err(err) => return Some(Err(ProjectRetrieveError::Name(err))),
        };

        let version = self
            .project_version(index)
            .expect("A version should be available, because a song is");

        Some(Ok(Project {
            name,
            version,
            song,
        }))
    }

    /// Retrieve the name of one of the projects _without decompressing it first_.
    ///
    /// If a project is not use, its name is non-sensical. [`None`] is returned (even though
    /// memory for a name may exist).
    ///
    /// Be aware that if you need all of a project's data, calling [`Self::project()`] directly is more efficient.
    pub fn project_name(&self, index: u5) -> Option<Result<Name<8>, NameFromBytesError>> {
        if self.is_project_in_use(index) {
            let offset = u8::from(index) as usize * 8;
            Some(Name::from_bytes(&self.blocks[offset..offset + 8]))
        } else {
            None
        }
    }

    /// Retrieve the version of one of the projects _without decompressing it first_.
    ///
    /// If a project is not use, its version is non-sensical. [`None`] is returned (even though
    /// memory for a version may exist).
    pub fn project_version(&self, index: u5) -> Option<u8> {
        if self.is_project_in_use(index) {
            let offset = 0x100 + u8::from(index) as usize;
            Some(self.blocks[offset])
        } else {
            None
        }
    }

    /// Decompress a project's song memory.
    ///
    /// If a project is not use, it doesn't have any compressed song data and [`None`] is returned.
    pub fn decompress_song_memory(&self, index: u5) -> Option<Result<SongMemory, io::Error>> {
        let index = index.into();
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

/// An error describing what could go wrong reading [`SRam`] from I/O
#[derive(Debug, Error)]
pub enum SRamReadError {
    /// All correctly initialized SRAM memory has certain magic bytes set.
    /// This error is returned when that isn't the case during an SRAM read.
    #[error("The SRAM initialization check failed")]
    CheckIncorrect,

    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),
}

/// An error describing what could go wrong retrieving a full [`Project`] from [`SRam`].
#[derive(Debug, Error)]
pub enum ProjectRetrieveError {
    /// All correctly initialized SRAM memory has certain magic bytes set.
    /// This error is returned when that isn't the case during an SRAM read.
    #[error("Retrieving the name failed")]
    Name(#[from] NameFromBytesError),

    /// Any failure that has to do with I/O
    #[error("Decompressing the song failed")]
    Decompression(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let sram = {
            let bytes = Cursor::new(include_bytes!("../../../test/92L_empty.sav"));
            SRam::from_reader(bytes).expect("could not parse SRAM")
        };

        assert_eq!(sram.active_project(), Some(u5::new(0)));

        assert!(sram.is_project_in_use(u5::new(0)));
        assert_eq!(
            sram.project_name(u5::new(0)),
            Some(Ok(Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap()))
        );
        assert_eq!(sram.project_version(u5::new(0)), Some(0));
        sram.decompress_song_memory(u5::new(0)).unwrap().unwrap();

        assert!(!sram.is_project_in_use(u5::new(1)));
        assert_eq!(sram.project_name(u5::new(1)), None);
        assert_eq!(sram.project_version(u5::new(1)), None);
        assert!(sram.decompress_song_memory(u5::new(1)).is_none());
    }
}
