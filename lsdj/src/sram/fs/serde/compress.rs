use super::utils::{read_byte, CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, DEFAULT_WAVE_BYTE, RLE_BYTE};
use crate::sram::song::{instrument::DEFAULT_INSTRUMENT, wave::DEFAULT_WAVE};
use std::{
    io::{BufRead, Read, Result, Seek, SeekFrom, Write},
    slice,
};
use system_interface::io::Peek;

pub fn compress_step<R, W>(mut reader: R, mut writer: W) -> Result<()>
where
    R: Read + Peek + BufRead + Seek,
    W: Write,
{
    if let count @ 1.. = count_matches(&mut reader, 0, &DEFAULT_INSTRUMENT)? {
        return writer.write_all(&[CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, count]);
    }

    if let count @ 1.. = count_matches(&mut reader, 0, &DEFAULT_WAVE)? {
        return writer.write_all(&[CMD_BYTE, DEFAULT_WAVE_BYTE, count]);
    }

    match read_byte(&mut reader)? {
        CMD_BYTE => writer.write_all(&[CMD_BYTE, CMD_BYTE])?,
        RLE_BYTE => writer.write_all(&[RLE_BYTE, RLE_BYTE])?,
        value => {
            let slice = slice::from_ref(&value);
            if let count @ 2.. = count_matches(&mut reader, 1, slice)? {
                writer.write_all(&[RLE_BYTE, value, count])?
            } else {
                writer.write_all(slice)?
            }
        }
    }

    Ok(())
}

fn count_matches<R>(mut reader: R, init: u8, slice: &[u8]) -> Result<u8>
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

fn matches_slice<R>(mut reader: R, slice: &[u8]) -> Result<bool>
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

fn matches_byte<R>(reader: R, byte: u8) -> Result<bool>
where
    R: Read + Peek,
{
    matches_slice(reader, slice::from_ref(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
        let mut dest = [0; 2];
        compress_step(Cursor::new([0xE0]), Cursor::new(dest.as_mut_slice())).unwrap();
        assert_eq!(dest, [0xE0, 0xE0]);
    }

    #[test]
    fn rle_literal() {
        let mut dest = [0; 2];
        compress_step(Cursor::new([0xC0]), Cursor::new(dest.as_mut_slice())).unwrap();
        assert_eq!(dest, [0xC0, 0xC0]);
    }

    #[test]
    fn rle() {
        let mut dest = [0; 3];
        compress_step(
            Cursor::new([4, 4, 4, 4, 4, 4, 4]),
            Cursor::new(dest.as_mut_slice()),
        )
        .unwrap();
        assert_eq!(dest, [0xC0, 4, 7]);
    }

    #[test]
    fn value() {
        let mut dest = [0; 1];
        compress_step(Cursor::new([4, 9]), Cursor::new(dest.as_mut_slice())).unwrap();
        assert_eq!(dest, [4]);
    }

    #[test]
    fn default_instrument() {
        let src = [
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
            0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0xFF,
        ];
        let mut dest = [0; 3];

        compress_step(Cursor::new(src), Cursor::new(dest.as_mut_slice())).unwrap();

        assert_eq!(dest, [0xE0, 0xF1, 0x02]);
    }

    #[test]
    fn default_wave() {
        let src = [
            0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55, 0x54, 0x43,
            0x32, 0x31, 0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76, 0x66, 0x55,
            0x54, 0x43, 0x32, 0x31, 0x8E, 0xCD, 0xCC, 0xBB, 0xAA, 0xA9, 0x99, 0x88, 0x87, 0x76,
            0x66, 0x55, 0x54, 0x43, 0x32, 0xFF,
        ];
        let mut dest = [0; 3];

        compress_step(Cursor::new(src), Cursor::new(dest.as_mut_slice())).unwrap();

        assert_eq!(dest, [0xE0, 0xF0, 0x02]);
    }
}
