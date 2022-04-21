// use super::utils::{CMD_BYTE, DEFAULT_INSTRUMENT_BYTE};
// use crate::sram::song::instrument::DEFAULT_INSTRUMENT;
// use std::io::{BufRead, Read, Result, Write};
// use system_interface::io::Peek;

// pub fn compress_step<R, W>(reader: R, mut writer: W) -> Result<()>
// where
//     R: Read + Peek,
//     W: Write,
// {
//     if matches(reader, &DEFAULT_INSTRUMENT)? {
//         writer.write_all(&[CMD_BYTE, DEFAULT_INSTRUMENT_BYTE, 1])?;
//     }

//     Ok(())
// }

// fn match_count<R>(mut reader: R, slice: &[u8]) -> Result<usize>
// where
//     R: Read + Peek + BufRead + Seek,
// {
//     let mut count = 0;
//     while matches(&mut reader, slice)? {
//         count += 1;
//         reader.
//     }
//     Ok(count)
// }

// fn matches<R>(mut reader: R, slice: &[u8]) -> Result<bool>
// where
//     R: Read + Peek,
// {
//     let mut dest = vec![0; slice.len()];
//     if reader.peek(&mut dest)? == slice.len() {
//         Ok(dest == slice)
//     } else {
//         Ok(false)
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io::Cursor;

//     #[test]
//     fn default_instrument() {
//         let src = [
//             0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
//             0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0x0,
//             0xA8, 0x0, 0x0, 0xFF, 0x0, 0x0, 0x3, 0x0, 0x0, 0xD0, 0x0, 0x0, 0x0, 0xF3, 0x0, 0xFF,
//         ];
//         let mut dest = [0; 3];

//         compress_step(Cursor::new(src), Cursor::new(dest.as_mut_slice())).unwrap();

//         assert_eq!(dest, [0xE0, 0xF1, 0x02]);
//     }
// }
