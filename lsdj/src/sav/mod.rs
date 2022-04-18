pub mod project;
pub mod serde;
pub mod song;

mod name;
pub use name::{FromBytesError, Name};

use crate::u5;
use song::SongMemory;
use std::io::{self, Read};

pub struct Sav {
    working_memory_song: SongMemory,
    blocks: [u8; 0x18000],
}

impl Sav {
    pub fn from_reader<R>(mut reader: R) -> io::Result<Self>
    where
        R: Read,
    {
        let mut wm = [0; SongMemory::LEN];
        reader.read_exact(wm.as_mut_slice())?;

        let mut blocks = [0; 0x18000];
        reader.read_exact(blocks.as_mut_slice())?;

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

    /// Return the part of block 0 that represents the block allocation table
    fn alloc_table(&self) -> &[u8] {
        &self.blocks[0x141..0x1FF]
    }
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

        assert!(!sav.is_project_in_use(1));
        assert_eq!(sav.project_name(1), None);
        assert_eq!(sav.project_version(1), None);
    }
}
