//! LittleSoundDJ SRAM/`.sav` file handling
//!
//! This module contains functionality for reading, writing and manipulating SRAM, which
//! is where LSDJ stores its songs. Usually people work with `.sav` files, which gameboy
//! emulators use to store the SRAM tied to a ROM. You can also download/upload `.sav`
//! files to flashcarts for playback on real hardware.

pub mod file;
pub mod lsdsng;
pub mod name;
pub mod song;

use file::filesystem::{Filesystem, FilesystemReadError};
use name::{FromBytesError, Name};
use song::{SongMemory, SongMemoryReadError};
use std::{
    fs::{create_dir_all, File},
    io::{self, Read, Write},
    path::Path,
};
use thiserror::Error;

/// A full representation of LittleSoundDJ SRAM
///
/// Every LSDJ save file consists of the same amount of bytes, in which both the song you're
/// currently working on is stored (uncompressed), as well as a filesystem containing at max
/// 32 (compressed) songs.
///
/// The first time you boot LSDJ it formats the SRAM to the expected structure, setting some
/// magic bytes for later verification as well. This crate allows you to do the same, but also
/// to deserialize [`SRam`] from disk or an arbitrary reader.
///
/// ```no_run
/// # use lsdj::sram::SRam;
/// # use std::fs::File;
/// // Construct valid SRAM with the default/empty song and an empty filesystem
/// let sram = SRam::new();
///
/// // Load SRAM from a path on disk
/// let sram = SRam::from_path("bangers.sav")?;
///
/// // Load SRAM from an arbitrary reader
/// let sram = SRam::from_reader(File::open("bangers.sav")?)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// In the same way, SRAM can be serialized back to the underlying byte structure:
///
/// ```no_run
/// # use lsdj::sram::SRam;
/// # use std::fs::File;
/// # let sram = SRam::new();
/// // Load SRAM from a path on disk
/// sram.to_path("bangers.sav")?;
///
/// // Load SRAM from an arbitrary reader
/// sram.to_writer(File::create("bangers.sav")?)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct SRam {
    /// The song that's currently being worked on in LSDJ
    pub working_memory_song: SongMemory,

    /// Compressed storage for songs not currently worked on
    pub filesystem: Filesystem,
}

impl SRam {
    /// Construct a new SRAM, with a default song and empty filesystem
    ///
    /// This function also sets some necessary verification bytes which LSDJ uses to check
    /// for corrupted memory
    pub fn new() -> Self {
        Self {
            working_memory_song: SongMemory::new(),
            filesystem: Filesystem::new(),
        }
    }

    /// Deserialize SRAM from an arbitrary I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, FromReaderError>
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

    /// Deserialize SRAM from a path on disk (.sav)
    pub fn from_path<P>(path: P) -> Result<Self, FromPathError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let sram = Self::from_reader(file)?;

        Ok(sram)
    }

    /// Serialize SRAM to an arbitrary I/O writer
    pub fn to_writer<W>(&self, mut writer: W) -> Result<(), io::Error>
    where
        W: Write,
    {
        self.working_memory_song.to_writer(&mut writer)?;
        self.filesystem.to_writer(writer)
    }

    /// Serialize SRAM to a path on disk (.sav)
    pub fn to_path<P>(&self, path: P) -> Result<(), io::Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        create_dir_all(path.parent().unwrap())?;
        self.to_writer(File::create(path)?)
    }
}

impl Default for SRam {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that might be returned from [`SRam::from_reader()`]
#[derive(Debug, Error)]
pub enum FromReaderError {
    /// Deserializing the file system from I/O failed
    #[error("Reading the filesystem failed")]
    Filesystem(#[from] FilesystemReadError),

    /// Deserializing the working memory song from I/O failed
    #[error("Reading the working memory song failed")]
    WorkingSong(#[from] SongMemoryReadError),
}

/// Errors that might be returned from [`SRam::from_path()`]
#[derive(Debug, Error)]
pub enum FromPathError {
    /// Opening the file itself failed
    #[error("Opening the file failed")]
    FileOpen(#[from] io::Error),

    /// Deserialization failed
    #[error("Reading the SRAM from file failed")]
    Read(#[from] FromReaderError),
}
