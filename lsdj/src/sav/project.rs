use super::{song::SongMemory, Name};

/// A song plus a name and version number
pub struct Project {
    pub name: Name<8>,
    pub version: u8,
    pub song: SongMemory,
}
