use super::{
    End,
    utils::{
        CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, DEFAULT_WAVE_BYTE, EOF_BYTE, RLE_BYTE, read_byte,
        write_repeated_byte, write_repeated_bytes,
    },
};
use crate::song::{instrument::DEFAULT_INSTRUMENT, wave::DEFAULT_WAVE};
use std::{
    io::{Read, Result, Seek, Write},
    slice,
};

/// Decompress data from an LSDJ block reader to an arbitrary I/O writer
///
/// This function reads bytes and decompresses them as described [here](https://littlesounddj.fandom.com/wiki/File_Management_Structure). The call
/// returns when either:
///
///  * An EOF byte has been read, ending the decompression algorithm. This returns [`End::EndOfFile`]
///  * A block jump command has been read, returning [`End::JumpToBlock`]
pub fn decompress_block<R, W>(mut reader: R, mut writer: W) -> Result<End>
where
    R: Read,
    W: Write + Seek,
{
    loop {
        match read_byte(&mut reader)? {
            RLE_BYTE => decompress_rle_byte(&mut reader, &mut writer)?,
            CMD_BYTE => match decompress_cmd_byte(&mut reader, &mut writer)? {
                CmdContinuation::Continue => (),
                CmdContinuation::End(continuation) => return Ok(continuation),
            },
            value => writer.write_all(slice::from_ref(&value))?,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CmdContinuation {
    Continue,
    End(End),
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

fn decompress_cmd_byte<R, W>(mut reader: R, mut writer: W) -> Result<CmdContinuation>
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
        EOF_BYTE => return Ok(CmdContinuation::End(End::EndOfFile)),
        block => return Ok(CmdContinuation::End(End::JumpToBlock(block))),
    }

    Ok(CmdContinuation::Continue)
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
            CmdContinuation::Continue
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
            CmdContinuation::Continue
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
            CmdContinuation::Continue
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
            CmdContinuation::End(End::JumpToBlock(4)),
        );
    }

    #[test]
    fn eof() {
        let mut plain = [0_u8; 1];

        assert_eq!(
            decompress_cmd_byte(Cursor::new([EOF_BYTE]), Cursor::new(plain.as_mut_slice()))
                .unwrap(),
            CmdContinuation::End(End::EndOfFile)
        );
    }
}
