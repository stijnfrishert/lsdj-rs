//! The LSDJ filesystem.
//!
//! Every SRAM comes with a filesystem to store (compressed) you're currently not working on.
//! This module contains files for manipulating such a filesystem, though you usually do not
//! have to construct one yourself.
//!
//! See [`SRam`](crate::sram) for more information.

mod filesystem;

pub use filesystem::{Entries, Entry, Filesystem, FromReaderError, Index};

use crate::{
    lsdsng::LsdSng,
    name::{FromBytesError, Name},
    serde::CompressBlockError,
    song::{self, SongMemory},
};
use thiserror::Error;

/// Something that consists of a [`Name`], version and _compressed_ [`SongMemory`].
///
/// LSDJ's [`SRam`](crate::sram) comes with a [`Filesystem`] for storing compressed
/// files. This trait represents their interface, where every file in the filesystem
/// has a name, a version number (which increases with every save) and compressed
/// song data.
///
/// Artists often export their files to actual OS-level files with the [`LsdSng`] format,
/// which represents just a single LSDJ song. This is why [`File`] is a trait, because
/// an [`LsdSng`] is technically also a [`File`].
pub trait File {
    /// The name of the song stored in the file
    fn name(&self) -> Result<Name<8>, FromBytesError>;

    /// The version (increased with every save) of the song
    fn version(&self) -> u8;

    /// Decompress the song stored in the file
    fn decompress(&self) -> Result<SongMemory, song::FromReaderError>;

    /// Decompress and combine all fields into an [`LsdSng`]
    fn lsdsng(&self) -> Result<LsdSng, FileToLsdSngError> {
        let name = self.name()?;
        let version = self.version();
        let song = self.decompress()?;

        Ok(LsdSng::from_song(name, version, &song)?)
    }
}

/// Errors that might occur converting a [`File`] to an [`LsdSng`]
#[derive(Debug, Error)]
pub enum FileToLsdSngError {
    /// Deserializing the name failed
    #[error("Deserializing the name failed")]
    Name(#[from] FromBytesError),

    /// Decompressing the song failed
    #[error("Decompessing the song failed")]
    Decompress(#[from] song::FromReaderError),

    /// (Re)compressing the song failed
    #[error("(Re)compressing the song failed")]
    Compress(#[from] CompressBlockError),
}
