pub struct Block([u8; Self::LEN]);

impl Block {
    pub const LEN: usize = 0x200;
}
