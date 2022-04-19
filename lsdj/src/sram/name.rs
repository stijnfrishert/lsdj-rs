//! A null-terminated/max-length string based on a subset of ASCII
use std::{fmt, str};
use thiserror::Error;

/// A null-terminated/max-length string based on a subset of ASCII
///
/// In several places in LSDJ save files (projects, instruments, speech synth) names
/// are encoded as null-terminated strings with a maximal character count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> Name<N> {
    // The special lightning bolt character (the actual glyph depends on your ROM)
    const LIGHTNING_BOLT_CHAR: u8 = 95;

    /// Try to convert a byte slice to a name
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NameFromBytesError> {
        if bytes.len() > N {
            return Err(NameFromBytesError::TooLong);
        }

        let mut dest = [0; N];
        for (index, byte) in bytes.iter().enumerate() {
            match *byte {
                byte if Self::is_byte_allowed(byte) => dest[index] = byte,
                0 => break,
                _ => return Err(NameFromBytesError::DisallowedByte { byte: *byte, index }),
            }
        }

        Ok(Self { bytes: dest })
    }

    /// Gain access to the underlying bytes that make up the name
    pub fn bytes(&self) -> &[u8; N] {
        &self.bytes
    }

    /// The number of characters in the name string
    pub fn len(&self) -> usize {
        self.bytes.iter().position(|c| *c == 0).unwrap_or(N)
    }

    /// Are there _any_ characters in the name string?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to a string
    pub fn as_str(&self) -> &str {
        // SAFETY: Safe, because in from_bytes we check whether any of the characters are allowed
        // (that is, a subset of ASCII) anyway
        unsafe { str::from_utf8_unchecked(&self.bytes[..self.len()]) }
    }

    /// Is a specific byte within the subset of ASCII usable for name strings?
    pub fn is_byte_allowed(byte: u8) -> bool {
        // The only allowed characters are the capitals A-Z, digits 0-9, space or the special
        // lightning bolt character
        (65..=90).contains(&byte)
            || (48..=57).contains(&byte)
            || byte == 20
            || byte == Self::LIGHTNING_BOLT_CHAR
    }
}

impl<const N: usize> Default for Name<N> {
    fn default() -> Self {
        Self { bytes: [0; N] }
    }
}

impl<const N: usize> fmt::Display for Name<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An error describing what could go wrong converting a byte slice to a [`Name`]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NameFromBytesError {
    /// Error case for when the source slice is too big to fit in the [`Name`] string.
    #[error("The slice did not fit in the name array")]
    TooLong,

    /// Only a specific subset of ASCII characters are allowed in [`Name`] strings.
    #[error("Byte {byte} at position {index} is not allowed as a name character")]
    DisallowedByte { byte: u8, index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes() {
        const HELLO: &str = "HELLO";

        let name = Name::<8>::from_bytes(HELLO.as_bytes()).expect("bytes rejected");
        assert_eq!(name.len(), 5);
        assert!(!name.is_empty());
        assert_eq!(name.as_str(), HELLO);
        assert_eq!(format!("{name}"), HELLO);

        assert_eq!(
            Name::<8>::from_bytes("123456789".as_bytes()),
            Err(NameFromBytesError::TooLong)
        );

        assert_eq!(
            Name::<8>::from_bytes("A!".as_bytes()),
            Err(NameFromBytesError::DisallowedByte {
                byte: 33, // '!'
                index: 1
            })
        );
    }

    #[test]
    fn default() {
        let name = Name::<8>::default();
        assert_eq!(name.len(), 0);
        assert!(name.is_empty());
        assert_eq!(name.as_str(), "");
    }
}
