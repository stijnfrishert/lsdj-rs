//! Anything having to do with LSDJ SRAM/save files (versus the ROM)

pub mod song;

mod fs;
mod name;
mod project;

pub use fs::{Filesystem, FilesystemReadError};
pub use name::{Name, NameFromBytesError};
pub use project::Project;

use song::SongMemory;
use std::io::{self, Read};
use thiserror::Error;

/// The SRAM for a full LSDJ save
pub struct SRam {
    pub working_memory_song: SongMemory,
    pub filesystem: Filesystem,
}

impl SRam {
    /// Parse SRAM from an I/O reader
    pub fn from_reader<R>(mut reader: R) -> Result<Self, SRamReadError>
    where
        R: Read,
    {
        let mut wm = [0; SongMemory::LEN];
        reader.read_exact(wm.as_mut_slice())?;

        let filesystem = Filesystem::from_reader(&mut reader)?;

        Ok(Self {
            working_memory_song: SongMemory(wm),
            filesystem,
        })
    }
}

/// An error describing what could go wrong reading [`SRam`] from I/O
#[derive(Debug, Error)]
pub enum SRamReadError {
    /// All correctly initialized SRAM memory has certain magic bytes set.
    /// This error is returned when that isn't the case during an SRAM read.
    #[error("Reading the filesystem failed")]
    Filesystem(#[from] FilesystemReadError),

    /// Any failure that has to do with I/O
    #[error("Something failed with I/O")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::u5;

    #[test]
    fn empty_92l() {
        use std::io::Cursor;

        let sram = {
            let bytes = Cursor::new(include_bytes!("../../../test/92L_empty.sav"));
            SRam::from_reader(bytes).expect("could not parse SRAM")
        };

        assert_eq!(sram.filesystem.active_file(), Some(u5::new(0)));

        assert!(sram.filesystem.is_file_in_use(u5::new(0)));
        assert_eq!(
            sram.filesystem.file_name(u5::new(0)),
            Some(Ok(Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap()))
        );
        assert_eq!(sram.filesystem.file_version(u5::new(0)), Some(0));
        sram.filesystem.file_contents(u5::new(0)).unwrap().unwrap();

        assert!(!sram.filesystem.is_file_in_use(u5::new(1)));
        assert_eq!(sram.filesystem.file_name(u5::new(1)), None);
        assert_eq!(sram.filesystem.file_version(u5::new(1)), None);
        assert!(sram.filesystem.file_contents(u5::new(1)).is_none());
    }
}
