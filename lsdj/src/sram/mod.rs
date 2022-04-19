//! Anything having to do with LSDJ SRAM/save files (versus the ROM)

pub mod fs;
pub mod lsdsng;
pub mod name;
pub mod song;

use fs::{Filesystem, FilesystemReadError};
use name::{Name, NameFromBytesError};
use song::{SongMemory, SongMemoryReadError};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
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
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SRamFromReaderError>
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

    /// Parse SRAM from a file (.sav)
    pub fn from_file<P>(path: P) -> Result<Self, SRamFromFileError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let sram = Self::from_reader(file)?;

        Ok(sram)
    }
}

/// An error describing what could go wrong reading [`SRam`] from I/O
#[derive(Debug, Error)]
pub enum SRamFromReaderError {
    // Reading the file system from I/O failed
    #[error("Reading the filesystem failed")]
    Filesystem(#[from] FilesystemReadError),

    // Reading the working memory song from I/O failed
    #[error("Reading the working memory song failed")]
    WorkingSong(#[from] SongMemoryReadError),
}

/// An error describing what could go wrong reading [`SRam`] from a file
#[derive(Debug, Error)]
pub enum SRamFromFileError {
    // Reading the file system from I/O failed
    #[error("Opening the file failed")]
    File(#[from] io::Error),

    // Reading the working memory song from I/O failed
    #[error("Reading the SRAM from file failed")]
    Read(#[from] SRamFromReaderError),
}
