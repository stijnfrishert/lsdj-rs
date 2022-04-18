use super::{instrument::DEFAULT_INSTRUMENT, wave::DEFAULT_WAVE, SongMemory};
use crate::sav::block::Block;
use std::{
    io::{Cursor, Read, Result, Seek, SeekFrom, Write},
    slice,
};

const RLE_BYTE: u8 = 0xC0;
const CMD_BYTE: u8 = 0xE0;
const DEFAULT_WAVE_BYTE: u8 = 0xF0;
const DEFAULT_INSTRUMENT_BYTE: u8 = 0xF1;
const EOF_BYTE: u8 = 0xFF;

// Blocks start at 0x8000 and take 0x200 bytes. The block from 0x8000 to 0x8200 is
// block 0. It isn't actually used for project data, but contains meta data about the
// other blocks. As such, the first project in a sav file will (almost always) start
// on block 1, or 0x8200.
const BLOCKS_OFFSET: usize = SongMemory::LEN;

pub fn decompress<R>(reader: R) -> Result<SongMemory>
where
    R: Read + Seek,
{
    let mut project = SongMemory::zeroed();
    let mut cursor = Cursor::new(project.as_mut_slice());

    decompress_until_eof(reader, &mut cursor)?;

    assert_eq!(cursor.stream_position()?, SongMemory::LEN as u64);

    Ok(project)
}

pub fn decompress_until_eof<R, W>(mut reader: R, mut writer: W) -> Result<()>
where
    R: Read + Seek,
    W: Write + Seek,
{
    loop {
        match read_byte(&mut reader)? {
            RLE_BYTE => decompress_rle_byte(&mut reader, &mut writer)?,
            CMD_BYTE => match decompress_cmd_byte(&mut reader, &mut writer)? {
                Continuation::Continue => (),
                Continuation::JumpToBlock(block) => {
                    reader.seek(SeekFrom::Start(block_position(block)))?;
                }
                Continuation::EndOfFile => break,
            },
            value => writer.write_all(slice::from_ref(&value))?,
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum Continuation {
    Continue,
    JumpToBlock(u8),
    EndOfFile,
}

fn block_position(block: u8) -> u64 {
    (BLOCKS_OFFSET + Block::LEN * block as usize) as u64
}

fn decompress_rle_byte<R, W>(mut reader: R, mut writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    match read_byte(&mut reader)? {
        RLE_BYTE => writer.write_all(&[RLE_BYTE])?,
        value => {
            let count = read_byte(reader)?;
            write_repeated_byte(value, count as usize, &mut writer)?
        }
    }

    Ok(())
}

fn decompress_cmd_byte<R, W>(mut reader: R, mut writer: W) -> Result<Continuation>
where
    R: Read,
    W: Write,
{
    match read_byte(&mut reader)? {
        CMD_BYTE => writer.write_all(&[CMD_BYTE])?,
        DEFAULT_WAVE_BYTE => {
            let count = read_byte(&mut reader)?;
            write_repeated_bytes(&DEFAULT_WAVE, count as usize, &mut writer)?
        }
        DEFAULT_INSTRUMENT_BYTE => {
            let count = read_byte(&mut reader)?;
            write_repeated_bytes(&DEFAULT_INSTRUMENT, count as usize, &mut writer)?
        }
        EOF_BYTE => return Ok(Continuation::EndOfFile),
        block => return Ok(Continuation::JumpToBlock(block)),
    }

    Ok(Continuation::Continue)
}

fn read_byte<R>(mut reader: R) -> Result<u8>
where
    R: Read,
{
    let mut byte = 0;
    reader.read_exact(slice::from_mut(&mut byte))?;
    Ok(byte)
}

fn write_repeated_byte<W>(value: u8, count: usize, writer: W) -> Result<()>
where
    W: Write,
{
    write_repeated_bytes(slice::from_ref(&value), count, writer)
}

fn write_repeated_bytes<W>(bytes: &[u8], count: usize, mut writer: W) -> Result<()>
where
    W: Write,
{
    for _ in 0..count {
        writer.write_all(bytes)?
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle() {
        let mut plain = [0_u8; 4];

        assert!(
            decompress_rle_byte(Cursor::new([0x11, 4]), Cursor::new(plain.as_mut_slice())).is_ok()
        );

        assert_eq!(plain, [0x11, 0x11, 0x11, 0x11]);
    }

    #[test]
    fn rle_literal() {
        let mut plain = [0_u8; 1];

        assert!(
            decompress_rle_byte(Cursor::new([RLE_BYTE]), Cursor::new(plain.as_mut_slice())).is_ok()
        );

        assert_eq!(plain, [0xC0]);
    }

    #[test]
    fn cmd_literal() {
        let mut plain = [0_u8; 1];

        assert_eq!(
            decompress_cmd_byte(Cursor::new([CMD_BYTE]), Cursor::new(plain.as_mut_slice()))
                .unwrap(),
            Continuation::Continue
        );

        assert_eq!(plain, [0xE0]);
    }

    #[test]
    fn default_wave() {
        let mut plain = [0_u8; 32];

        assert_eq!(
            decompress_cmd_byte(
                Cursor::new([DEFAULT_WAVE_BYTE, 2]),
                Cursor::new(plain.as_mut_slice())
            )
            .unwrap(),
            Continuation::Continue
        );

        assert_eq!(
            plain,
            [
                0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55, 0x54, 0x43,
                0x32, 0x31, 0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55,
                0x54, 0x43, 0x32, 0x31
            ]
        );
    }

    #[test]
    fn default_instrument() {
        let mut plain = [0_u8; 32];

        assert_eq!(
            decompress_cmd_byte(
                Cursor::new([DEFAULT_INSTRUMENT_BYTE, 2]),
                Cursor::new(plain.as_mut_slice())
            )
            .unwrap(),
            Continuation::Continue
        );

        assert_eq!(
            plain,
            [
                0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
                0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
            ]
        );
    }

    #[test]
    fn block_jump() {
        let mut plain = [0_u8; 1];

        assert_eq!(
            decompress_cmd_byte(Cursor::new([4]), Cursor::new(plain.as_mut_slice())).unwrap(),
            Continuation::JumpToBlock(4),
        );
    }

    #[test]
    fn eof() {
        let mut plain = [0_u8; 1];

        assert_eq!(
            decompress_cmd_byte(Cursor::new([EOF_BYTE]), Cursor::new(plain.as_mut_slice()))
                .unwrap(),
            Continuation::EndOfFile
        );
    }

    #[test]
    fn empty() {
        let sav = include_bytes!("../../../../test/92L_empty.sav");
        let mut reader = Cursor::new(sav);

        reader
            .seek(SeekFrom::Start(block_position(1)))
            .expect("could not seek to blocks offset");

        assert!(decompress(reader).is_ok());
    }
}
