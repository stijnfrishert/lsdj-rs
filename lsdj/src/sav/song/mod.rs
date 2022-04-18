pub mod instrument;
pub mod wave;

/// A contiguous block of memory that represents unparsed song data.
pub struct SongMemory(pub(super) [u8; Self::LEN]);

impl SongMemory {
    /// The number of bytes taken up by a single LSDJ song
    pub const LEN: usize = 0x8000;
}
