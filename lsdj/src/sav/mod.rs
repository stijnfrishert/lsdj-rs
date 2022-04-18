pub mod name;
pub mod project;
pub mod serde;
pub mod song;

use name::{FromBytesError, Name};
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
    pub fn active_project(&self) -> Option<u8> {
        match self.blocks[0x140] {
            0xFF => None,
            index => Some(index),
        }
    }

    pub fn project_name(&self, index: u8) -> Result<Name<8>, FromBytesError> {
        let offset = index as usize * 8;
        Name::from_bytes(&self.blocks[offset..offset + 8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        use std::io::Cursor;

        let sav = Sav::from_reader(Cursor::new(include_bytes!("../../../test/92L_empty.sav")))
            .expect("could not load sav");

        assert_eq!(sav.active_project(), Some(0));

        assert_eq!(
            sav.project_name(0),
            Ok(Name::<8>::from_bytes("EMPTY".as_bytes()).unwrap())
        );
    }
}
