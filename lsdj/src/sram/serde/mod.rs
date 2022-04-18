//! Functionality for serializing and deserializing songs into LSDJ's block structure

mod decompress;

pub use decompress::decompress;

/// The length in bytes of a compression block
pub const BLOCK_LEN: usize = 0x200;
