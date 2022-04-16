pub mod block;
pub mod compress;
pub mod decompress;
pub mod instrument;
pub mod wave;

pub struct Project([u8; Self::LEN]);

impl Project {
    pub const LEN: usize = 0x8000;

    pub fn zeroed() -> Self {
        Self([0; Self::LEN])
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
}
