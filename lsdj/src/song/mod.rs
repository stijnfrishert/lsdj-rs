//! Unparsed LSDJ song memory

pub(crate) mod instrument;
pub(crate) mod wave;

pub mod v22;

mod song_memory;

pub use song_memory::{FromBytesError, FromReaderError, SongMemory};

use thiserror::Error;

pub enum Song {
    V22(v22::Song),
}

impl Song {
    pub fn from_memory(memory: &SongMemory) -> Result<Self, SongFromMemoryError> {
        let version = memory.format_version();

        match version {
            22 => Ok(Self::V22(v22::Song::from_memory(memory))),
            _ => Err(SongFromMemoryError::UnsupportedVersion(version)),
        }
    }
}

#[derive(Debug, Error)]
pub enum SongFromMemoryError {
    #[error("Song memory contained version {0}, which is currently not supported")]
    UnsupportedVersion(u8),
}
