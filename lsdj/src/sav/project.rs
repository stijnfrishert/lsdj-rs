use super::{song::SongMemory, Name};

pub struct Project {
    pub name: Name<8>,
    pub version: u8,
    pub song: SongMemory,
}
