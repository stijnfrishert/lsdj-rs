pub mod compress;
pub mod decompress;
mod utils;

#[derive(Debug, PartialEq, Eq)]
pub enum End {
    JumpToBlock(u8),
    EndOfFile,
}
