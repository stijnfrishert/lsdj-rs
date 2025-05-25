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

#[derive(Clone)]
pub struct Sample {
    name: String,
    data: Vec<u8>,
}

impl Display for Sample {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Name: {}, Length: {}", self.name, self.data.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::Kit;

    #[test]
    fn test_parse_kit() {
        let kit = Kit::try_from(include_bytes!("../../test/snap.kit").to_vec())
            .expect("Failed to parse Kit file");
        assert_eq!(kit.name, "SNAP  ");
        assert_eq!(kit.version, 1);
        assert_eq!(kit.samples.len(), 1);
        assert_eq!(kit.samples[0].name, "SNA");
    }
}
