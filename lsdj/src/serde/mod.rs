//! Implementation of the [LSDJ compression algorithm](https://littlesounddj.fandom.com/wiki/File_Management_Structure)

mod compress;
mod decompress;
mod utils;

pub use compress::{CompressBlockError, compress_block};
pub use decompress::decompress_block;

/// The result of block compression/decompression
///
/// See [`compress_block`] and [`decompress_block`] for more information on when this is returned
#[derive(Debug, PartialEq, Eq)]
pub enum End {
    /// A block-jump command has been written/read
    JumpToBlock(u8),

    /// An EOF command has been written/read
    EndOfFile,
}
