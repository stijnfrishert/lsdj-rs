pub mod decompress;
pub mod instrument;
pub mod wave;

/// A contiguous block of memory that represents unparsed song data.
pub struct SongMemory([u8; Self::LEN]);

impl SongMemory {
    /// The number of bytes taken up by a single LSDJ song
    pub const LEN: usize = 0x8000;

    /// Create a block of song memory made entirely of 0's
    ///
    /// Note that this does not comprise a proper song structure
    pub(crate) fn zeroed() -> Self {
        Self([0; Self::LEN])
    }

    /// Get immutable access to the bytes in the song
    pub(crate) fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Get mutable access to the bytes in the song
    pub(crate) fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
}
