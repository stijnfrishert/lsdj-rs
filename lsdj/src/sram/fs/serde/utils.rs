use std::{
    io::{Read, Result, Write},
    slice,
};

pub const RLE_BYTE: u8 = 0xC0;
pub const CMD_BYTE: u8 = 0xE0;
pub const DEFAULT_WAVE_BYTE: u8 = 0xF0;
pub const DEFAULT_INSTRUMENT_BYTE: u8 = 0xF1;
pub const EOF_BYTE: u8 = 0xFF;

pub fn read_byte<R>(mut reader: R) -> Result<u8>
where
    R: Read,
{
    let mut byte = 0;
    reader.read_exact(slice::from_mut(&mut byte))?;
    Ok(byte)
}

pub fn write_repeated_byte<W>(value: u8, count: usize, writer: W) -> Result<()>
where
    W: Write,
{
    write_repeated_bytes(slice::from_ref(&value), count, writer)
}

pub fn write_repeated_bytes<W>(bytes: &[u8], count: usize, mut writer: W) -> Result<()>
where
    W: Write,
{
    for _ in 0..count {
        writer.write_all(bytes)?
    }

    Ok(())
}
