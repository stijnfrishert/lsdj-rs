use crate::sram::song::{instrument::DEFAULT_INSTRUMENT, wave::DEFAULT_WAVE};
use std::{
    io::{Read, Result, Seek, Write},
    slice,
};

const RLE_BYTE: u8 = 0xC0;
const CMD_BYTE: u8 = 0xE0;
const DEFAULT_WAVE_BYTE: u8 = 0xF0;
const DEFAULT_INSTRUMENT_BYTE: u8 = 0xF1;
const EOF_BYTE: u8 = 0xFF;

pub fn decompress_block<R, W>(mut reader: R, mut writer: W) -> Result<Option<u8>>
where
    R: Read,
    W: Write + Seek,
{
    loop {
        match read_byte(&mut reader)? {
            RLE_BYTE => decompress_rle_byte(&mut reader, &mut writer)?,
            CMD_BYTE => match decompress_cmd_byte(&mut reader, &mut writer)? {
                Continuation::Continue => (),
                Continuation::JumpToBlock(block) => return Ok(Some(block)),
                Continuation::EndOfFile => return Ok(None),
            },
            value => writer.write_all(slice::from_ref(&value))?,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Continuation {
    Continue,
    JumpToBlock(u8),
    EndOfFile,
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
    use std::io::Cursor;

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
}
