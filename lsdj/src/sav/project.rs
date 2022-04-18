use super::song::SongMemory;
use str_buf::StrBuf;

pub struct Project {
    pub name: StrBuf<8>,
    pub version: u8,
    pub song: SongMemory,
}
