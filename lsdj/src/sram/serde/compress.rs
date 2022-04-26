use super::{
    utils::{read_byte, CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, DEFAULT_WAVE_BYTE, RLE_BYTE},
    End,
};
use crate::sram::song::{instrument::DEFAULT_INSTRUMENT, wave::DEFAULT_WAVE};
use std::{
    io::{self, BufRead, Read, Seek, SeekFrom, Write},
    slice,
};
use system_interface::io::Peek;
use thiserror::Error;

/// Compress data from an I/O reader into an LSDJ block
///
/// This function reads bytes and compresses them as described [here](https://littlesounddj.fandom.com/wiki/File_Management_Structure). The call
/// returns when either:
///
///  * The end of the reader has been reached, which returns [`End::EndOfFile`]
///  * The block is full. `next_block()` is called for retrieve the index of the next block, and [`End::JumpToBlock`] is returned.
pub fn compress_block<R, W, F>(
    mut reader: R,
    mut writer: W,
    next_block: F,
) -> Result<End, CompressBlockError>
where
    R: Read + BufRead + Seek,
    W: Write + Seek,
    F: FnOnce() -> Option<u8>,
{
    let read_end = end(&mut reader)?;
    let write_end = end(&mut writer)?;

    loop {
        let write_pos = writer.stream_position()?;
        let left = write_end - write_pos;

        // Check if we've reached the end-of-file
        if reader.stream_position()? == read_end {
            writer.write_all(&[0xE0, 0xFF])?;
            writer.write_all(&vec![0; (left - 2) as usize])?;
            return Ok(End::EndOfFile);
        }

        if left >= 5 {
            let compression = compress_step(&mut reader)?;
            compression.write(&mut writer)?;
        } else {
            let index = next_block().ok_or(CompressBlockError::NoBlockLeft)?;
            writer.write_all(&[0xE0, index])?;
            writer.write_all(&vec![0; (left - 2) as usize])?;
            return Ok(End::JumpToBlock(index));
        }
    }
}

/// Errors that might be returned from [`compress_block()`]
#[derive(Debug, Error)]
pub enum CompressBlockError {
    // Something went wrong with reading or writing from I/O
    #[error("Reading/writing from I/O failed")]
    Io(#[from] io::Error),

    // There are no more empty blocks left to continue to
    #[error("The filesystem ran out of blocks")]
    NoBlockLeft,
}

fn end<S>(mut seeker: S) -> io::Result<u64>
where
    S: Seek,
{
    let pos = seeker.stream_position()?;
    seeker.seek(SeekFrom::End(0))?;
    let end = seeker.stream_position()?;
    seeker.seek(SeekFrom::Start(pos))?;
    Ok(end)
}

fn compress_step<R>(mut reader: R) -> io::Result<Compression>
where
    R: Read + Peek + BufRead + Seek,
{
    if let count @ 1.. = count_matches(&mut reader, 0, &DEFAULT_INSTRUMENT)? {
        return Ok(Compression::DefaultInstrument { count });
    }

    if let count @ 1.. = count_matches(&mut reader, 0, &DEFAULT_WAVE)? {
        return Ok(Compression::DefaultWave { count });
    }

    match read_byte(&mut reader)? {
        CMD_BYTE => Ok(Compression::CmdLiteral),
        RLE_BYTE => Ok(Compression::RleLiteral),
        value => {
            if let count @ 2.. = count_matches(&mut reader, 1, slice::from_ref(&value))? {
                Ok(Compression::RunLengthEncoding { value, count })
            } else {
                Ok(Compression::Literal { value })
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Compression {
    RunLengthEncoding { value: u8, count: u8 },
    DefaultInstrument { count: u8 },
    DefaultWave { count: u8 },
    RleLiteral,
    CmdLiteral,
    Literal { value: u8 },
}

impl Compression {
    pub fn write<W>(self, mut writer: W) -> io::Result<()>
    where
        W: Write,
    {
        match self {
            Self::RunLengthEncoding { value, count } => writer.write_all(&[RLE_BYTE, value, count]),
            Self::DefaultInstrument { count } => {
                writer.write_all(&[CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, count])
            }
            Self::DefaultWave { count } => writer.write_all(&[CMD_BYTE, DEFAULT_WAVE_BYTE, count]),
            Self::RleLiteral => writer.write_all(&[RLE_BYTE, RLE_BYTE]),
            Self::CmdLiteral => writer.write_all(&[CMD_BYTE, CMD_BYTE]),
            Self::Literal { value } => writer.write_all(&[value]),
        }
    }
}

fn count_matches<R>(mut reader: R, init: u8, slice: &[u8]) -> io::Result<u8>
where
    R: Read + Peek + BufRead + Seek,
{
    let mut count = init;
    while matches_slice(&mut reader, slice)? && count < u8::MAX {
        count += 1;
        reader.seek(SeekFrom::Current(slice.len() as i64))?;
    }
    Ok(count)
}

fn matches_slice<R>(mut reader: R, slice: &[u8]) -> io::Result<bool>
where
    R: Read + Peek,
{
    let mut dest = vec![0; slice.len()];
    if reader.peek(&mut dest)? == slice.len() {
        Ok(dest == slice)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn assert_write<const N: usize>(compression: Compression, expected: [u8; N]) {
        let mut dest = [0; N];
        compression.write(Cursor::new(dest.as_mut_slice())).unwrap();
        assert_eq!(&dest, &expected);
    }

    #[test]
    fn matches() {
        assert!(matches_slice(Cursor::new([0, 1]), &[0, 1]).unwrap());
        assert!(!matches_slice(Cursor::new([0, 1]), &[0, 4]).unwrap());

        assert_eq!(
            count_matches(Cursor::new([5, 5, 5, 5, 6]), 0, &[5, 5]).unwrap(),
            2
        );
    }

    #[test]
    fn cmd_literal() {
        let compression = compress_step(Cursor::new([0xE0])).unwrap();
        assert_eq!(compression, Compression::CmdLiteral);
        assert_write(compression, [0xE0, 0xE0]);
    }

    #[test]
    fn rle_literal() {
        let compression = compress_step(Cursor::new([0xC0])).unwrap();
        assert_eq!(compression, Compression::RleLiteral);
        assert_write(compression, [0xC0, 0xC0]);
    }

    #[test]
    fn rle() {
        let compression = compress_step(Cursor::new([4, 4, 4, 4, 4, 4, 4])).unwrap();
        assert_eq!(
            compression,
            Compression::RunLengthEncoding { value: 4, count: 7 }
        );
        assert_write(compression, [0xC0, 0x04, 0x07]);
    }

    #[test]
    fn value() {
        let compression = compress_step(Cursor::new([4, 9])).unwrap();
        assert_eq!(compression, Compression::Literal { value: 4 });
        assert_write(compression, [0x04]);
    }

    #[test]
    fn default_instrument() {
        let compression = compress_step(Cursor::new([
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0xFF,
        ]))
        .unwrap();
        assert_eq!(compression, Compression::DefaultInstrument { count: 2 });
        assert_write(compression, [0xE0, 0xF1, 0x02]);
    }

    #[test]
    fn default_wave() {
        let compression = compress_step(Cursor::new([
            0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55, 0x54, 0x43,
            0x32, 0x31, 0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55,
            0x54, 0x43, 0x32, 0x31, 0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76,
            0x66, 0x55, 0x54, 0x43, 0x32, 0xFF,
        ]))
        .unwrap();
        assert_eq!(compression, Compression::DefaultWave { count: 2 });
        assert_write(compression, [0xE0, 0xF0, 0x02]);
    }

    #[test]
    fn block() {
        let mut reader = Cursor::new([4, 4, 4, 9]);

        let mut dest = [0; 10];
        let end = compress_block(&mut reader, Cursor::new(&mut dest[..5]), || Some(1));
        assert_eq!(end.unwrap(), End::JumpToBlock(1));

        let end = compress_block(reader, Cursor::new(&mut dest[5..]), || None);
        assert_eq!(end.unwrap(), End::EndOfFile);

        assert_eq!(dest, [0xC0, 4, 3, 0xE0, 1, 9, 0xE0, 0xFF, 0x0, 0x0]);
    }
}
