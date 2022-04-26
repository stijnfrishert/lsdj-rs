//! Tools for working with the concept of LSDJ files (lsdsng, the compressed files in the SRAM, etc.)

pub mod filesystem;
pub mod serde;

use crate::sram::{
    file::serde::CompressBlockError,
    lsdsng::LsdSng,
    name::{FromBytesError, Name},
    song::{FromReaderError, SongMemory},
};
use thiserror::Error;

/// A [`File`] is a compressed LSDJ song + a name and version
///
/// Files are most commonly stored in the [`SRam`](crate::sram)'s filesystem,
/// but an [`LsdSng`] is also a good example of a file.
pub trait File {
    fn name(&self) -> Result<Name<8>, FromBytesError>;
    fn version(&self) -> u8;
    fn decompress(&self) -> Result<SongMemory, FromReaderError>;

    fn lsdsng(&self) -> Result<LsdSng, FileToLsdSngError> {
        let name = self.name()?;
        let version = self.version();
        let song = self.decompress()?;

        Ok(LsdSng::from_song(name, version, &song)?)
    }
}

/// An error describing what could go wrong convering a [`File`] to an [`LsdSng`]
#[derive(Debug, Error)]
pub enum FileToLsdSngError {
    /// All correctly initialized filesystem memory has certain magic bytes set.
    /// This error is returned when that isn't the case during a read.
    #[error("The initialization check failed")]
    Name(#[from] FromBytesError),

    /// Decompressing the song failed
    #[error("Decompessing the song failed")]
    Decompress(#[from] FromReaderError),

    /// Compressing the song failed
    #[error("Compressing the song failed")]
    Compress(#[from] CompressBlockError),
}
