//! The .lsdsng format

use crate::sram::name::{Name, NameFromBytesError};
use std::{
    io::{self, Read},
    slice,
};
use thiserror::Error;

/// A song + a name and version
///
/// This is often used to export and import songs from/to [`SRam`]/save files
pub struct LsdSng {
    pub name: Name<8>,
    pub version: u8,
    pub blocks: Vec<u8>,
}

impl LsdSng {
    pub fn from_reader<R>(mut reader: R) -> Result<LsdSng, LsdsngFromReaderError>
    where
        R: Read,
    {
        let name = {
            let mut bytes = [0; 8];
            reader.read_exact(&mut bytes)?;
            Name::from_bytes(bytes.as_mut_slice())?
        };

        let mut version = 0;
        reader.read_exact(slice::from_mut(&mut version))?;

        let mut blocks = Vec::new();
        reader.read_to_end(&mut blocks)?;

        Ok(LsdSng {
            name,
            version,
            blocks,
        })
    }
}

/// An error describing what could go wrong reading an [`LsdSng`] from I/O
#[derive(Debug, Error)]
pub enum LsdsngFromReaderError {
    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),

    /// Could not read the name successfully
    #[error("Reading the name failed")]
    Name(#[from] NameFromBytesError),
}
