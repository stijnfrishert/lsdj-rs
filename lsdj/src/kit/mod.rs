use std::fmt::{Display, Formatter};
use std::io::Read;

const MAX_SAMPLE_SPACE_PER_BANK: usize = 0x3fa0;
const BANK_SIZE: usize = 0x4000;
const MAX_SAMPLES_PER_BANK: usize = 15;
const SAMPLE_NAME_OFFSET: usize = 0x22;
const SAMPLE_NAME_LENGTH: usize = 3;
const KIT_NAME_OFFSET: usize = 0x52;
const KIT_NAME_LENGTH: usize = 6;
const KIT_VERSION_OFFSET: usize = 0x5f;
const FORCE_LOOP_OFFSET: usize = 0x5c;

pub struct Kit {
    name: String,
    version: u8,
    samples: Vec<Sample>,
}

impl Kit {
    pub fn try_from_reader<R: Read>(mut r: R) -> Result<Kit, String> {
        let mut buf = Vec::new();
        match r.read_to_end(&mut buf) {
            Ok(_) => Kit::try_from(buf),
            Err(e) => Err(e.to_string()),
        }
    }
}
impl Display for Kit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match write!(
            f,
            "Kit {{name: {}, version: {}}}\n",
            self.name, self.version
        ) {
            Ok(_) => {}
            Err(_) => return Err(std::fmt::Error),
        };
        self.samples
            .iter()
            .map(|s| write!(f, "{}\n", s))
            .collect::<std::fmt::Result>()
    }
}

fn parse_force_loop_bits(bytes: &[u8]) -> Vec<bool> {
    debug_assert_eq!(bytes.len(), 2);
    bytes.iter().flat_map(|v| {
        let a = v & 0x01 == 0x01;
        let b = v & 0x02 == 0x02;
        let c = v & 0x04 == 0x04;
        let d = v & 0x08 == 0x08;
        let e = v & 0x10 == 0x10;
        let f = v & 0x20 == 0x20;
        let g = v & 0x40 == 0x40;
        let h = v & 0x80 == 0x80;
        return vec![h, g, f, e, d, c, b, a];
    }).collect()
}

fn force_loop_bits_to_flags(flags: [bool; 16]) -> [u8; 2] {
    debug_assert_eq!(flags.len(), 16);
    let mut a: u8 = 0x00;
    let mut b: u8 = 0x00;
    for i in 0..16 {
        if flags[i] {
            if i < 8 {
                a |= 0x01 << i;
            } else {
                b |= 0x01 << i-8;
            }
        }
    }
    [a, b]
}

impl TryFrom<Vec<u8>> for Kit {
    type Error = String;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        if v.len() != BANK_SIZE {
            return Err(format!("Invalid Kit size: 0x{:X}", v.len()));
        }
        if v[0] != 0x60 || v[1] != 0x40 {
            return Err(format!("Invalid Kit header: 0x{:X} 0x{:X}", v[0], v[1]));
        }
        let name =
            match String::from_utf8(v[KIT_NAME_OFFSET..KIT_NAME_OFFSET + KIT_NAME_LENGTH].to_vec())
            {
                Ok(s) => s,
                Err(_) => return Err("Kit name invalid".to_string()),
            };
        let version = v[KIT_VERSION_OFFSET];
        let force_loop_bits = parse_force_loop_bits(
            &v[FORCE_LOOP_OFFSET..FORCE_LOOP_OFFSET + 2]
        );
        let samples = match (0..MAX_SAMPLES_PER_BANK).map(|track| {
            let name_offset = SAMPLE_NAME_OFFSET + (track * SAMPLE_NAME_LENGTH);
            let name = match String::from_utf8(v[name_offset..name_offset+3].to_vec()) {
               Ok(s) => s,
               Err(_) => return Err(format!("Sample {track} name invalid"))
            };
            let data = {
                let i = track*2;
                let mut offset_start = ((v[i] as u16) | (v[i +1] as u16) << 8) as usize;
                if offset_start == 0x00 || offset_start == 0xFFFF {
                    return Ok(None)
                }
                offset_start -= BANK_SIZE;
                let mut offset_end = ((v[i +2] as u16) | (v[i +3] as u16) << 8) as usize;
                if offset_end == 0x00 || offset_end == 0xFFFF {
                    return Ok(None)
                }
                offset_end -= BANK_SIZE;
                if offset_start > offset_end {
                    return Err(format!(
                        "Sample {track} offset table entry ends before it starts: \
                        offset_start=0x{offset_start:X} offset_end=0x{offset_end:X}"))
                }
                if offset_end > MAX_SAMPLE_SPACE_PER_BANK {
                    return Err(format!(
                        "Sample {track} offset table entry ends after the maximum sample space: \
                        offset_start=0x{offset_start:X} offset_end=0x{offset_end:X}"))
                }
                v[offset_start..offset_end].to_vec()
            };
            Ok(Some(Sample {
                name,
                data,
                force_loop: force_loop_bits[track],
            }))
        })
            .collect::<Result<Vec<Option<Sample>>, String>>() {
            Ok(s) => s,
            Err(e) => return Err(e),
        }.iter()
            .filter_map(|x| x.clone())
            .collect();
        Ok(Kit {
            name,
            version,
            samples,
        })
    }
}

impl TryFrom<&[u8]> for Kit {
    type Error = String;
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        Kit::try_from(v.to_vec())
    }
}

impl Into<[u8; BANK_SIZE]> for Kit {
    fn into(self) -> [u8; BANK_SIZE] {
        let mut bytes: [u8; BANK_SIZE] = [0xFF; BANK_SIZE];
        let name = self.name.to_ascii_uppercase().as_bytes().to_owned();
        bytes[KIT_NAME_OFFSET..KIT_NAME_OFFSET + KIT_NAME_LENGTH]
            .copy_from_slice(&name[..6]);
        let force_loop_bits = self.samples.iter().map(|s| s.force_loop)
            .collect::<Vec<bool>>();
        let mut flags = [false; 16];
        flags[0..force_loop_bits.len()].copy_from_slice(&force_loop_bits.as_slice());
        bytes[FORCE_LOOP_OFFSET..FORCE_LOOP_OFFSET + 2].copy_from_slice(
            force_loop_bits_to_flags(flags).as_ref()
        );
        bytes[KIT_VERSION_OFFSET] = self.version;
        let mut sample_offset: u16 = 0x60;
        let mut sample_index: usize = 0;
        bytes[0..2].copy_from_slice((sample_offset+BANK_SIZE as u16).to_le_bytes().as_ref());
        for sample in self.samples.iter() {
            let mut name: [u8; SAMPLE_NAME_LENGTH] = [0x00; SAMPLE_NAME_LENGTH];
            name.copy_from_slice(sample.name.to_ascii_uppercase().as_bytes()[0..SAMPLE_NAME_LENGTH].as_ref());
            bytes[
                SAMPLE_NAME_OFFSET+sample_index*SAMPLE_NAME_LENGTH..
                    SAMPLE_NAME_OFFSET+(sample_index+1)*SAMPLE_NAME_LENGTH
                ].copy_from_slice(name.as_ref());
            sample_index += 1;
            // Copy sample data.
            bytes[sample_offset as usize..sample_offset as usize+sample.data.len()]
                .copy_from_slice(&sample.data);
            sample_offset += sample.data.len() as u16;
            // Add end track marker - beginning of next track.
            bytes[sample_index*2..sample_index*2+2]
                .copy_from_slice((sample_offset+BANK_SIZE as u16).to_le_bytes().as_ref());
        }
        for i in sample_index..MAX_SAMPLES_PER_BANK {
            bytes[
                SAMPLE_NAME_OFFSET+i*SAMPLE_NAME_LENGTH..
                    SAMPLE_NAME_OFFSET+(i+1)*SAMPLE_NAME_LENGTH
                ].copy_from_slice(&[0x00, '-' as u8, '-' as u8])
        }
        sample_index += 1;
        // Zero the remainder.
        bytes[sample_index*2..0x20].fill(0);
        bytes
    }
}

#[derive(Clone)]
pub struct Sample {
    name: String,
    data: Vec<u8>,
    force_loop: bool,
}

impl Display for Sample {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Name: {}, Length: {}, Force Loop: {}", 
               self.name, self.data.len(), self.force_loop)
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::{force_loop_bits_to_flags, parse_force_loop_bits, Kit, BANK_SIZE};

    #[test]
    fn test_parse_kit() {
        let kit = Kit::try_from(include_bytes!("../../test/snap.kit").to_vec())
            .expect("Failed to parse Kit file");
        assert_eq!(kit.name, "SNAP  ");
        assert_eq!(kit.version, 1);
        assert_eq!(kit.samples.len(), 1);
        assert_eq!(kit.samples[0].name, "SNA");
        assert_eq!(kit.samples[0].force_loop, false);
    }

    #[test]
    fn test_parse_force_loop_bits() {
        assert_eq!(parse_force_loop_bits(&[0xFF, 0xFF]), vec![true; 16]);
        assert_eq!(parse_force_loop_bits(&[0x00, 0x00]), vec![false; 16]);
        let mut first_true = vec![false; 16];
        first_true[0] = true;
        assert_eq!(parse_force_loop_bits(&[0x80, 0x00]), first_true);
    }
    
    #[test]
    fn test_force_loop_bits_to_flags() {
        let all_true = [true; 16];
        assert_eq!(force_loop_bits_to_flags(all_true).to_vec(), vec![0xFF, 0xFF]);
        let all_false = [false; 16];
        assert_eq!(force_loop_bits_to_flags(all_false).to_vec(), vec![0x00, 0x00]);
    }
    
    #[test]
    fn test_into_u8() {
        let snap = *include_bytes!("../../test/snap.kit");
        let kit = Kit::try_from(snap.to_vec()).expect("Failed to parse Kit file");
        let snap2: [u8; BANK_SIZE] = kit.into();
        assert_eq!(snap, snap2)
    }
}
