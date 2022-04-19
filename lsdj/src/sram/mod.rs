//! Anything having to do with LSDJ SRAM/save files (versus the ROM)

pub mod fs;
pub mod name;
pub mod song;

use fs::{Filesystem, FilesystemReadError};
use name::{Name, NameFromBytesError};
use song::{SongMemory, SongMemoryReadError};
use std::io::Read;
use thiserror::Error;

/// The SRAM for a full LSDJ save
pub struct SRam {
    /// The song that's currently being worked on in LSDJ
    pub working_memory_song: SongMemory,

    /// Compressed storage for songs not currently worked on
    pub filesystem: Filesystem,
}

impl SRam {
    /// Parse SRAM from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SRamReadError>
    where
        R: Read,
    {
        let working_memory_song = SongMemory::from_reader(&mut reader)?;
        let filesystem = Filesystem::from_reader(&mut reader)?;

        Ok(Self {
            working_memory_song,
            filesystem,
        })
    }
}

/// An error describing what could go wrong reading [`SRam`] from I/O
#[derive(Debug, Error)]
pub enum SRamReadError {
    // Reading the file system from I/O failed
    #[error("Reading the filesystem failed")]
    Filesystem(#[from] FilesystemReadError),

    // Reading the working memory song from I/O failed
    #[error("Reading the working memory song failed")]
    WorkingSong(#[from] SongMemoryReadError),
}
