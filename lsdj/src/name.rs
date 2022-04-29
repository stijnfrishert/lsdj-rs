//! A null-terminated/length-restricted string based on a subset of ASCII
use std::{
    fmt,
    str::{self, FromStr},
};
use thiserror::Error;

/// A null-terminated/length-restricted string based on a subset of ASCII
///
/// Several LSDJ structures have names (e.g. files and instruments), which are
/// encoded as null-terminated strings with a maximal length (think [strnlen](https://en.cppreference.com/w/c/string/byte/strlen)).
///
/// The maximum length isn't the same everywhere, which is why this struct is generic over its length.
///
/// The allowed characters in a [`Name`] are (ASCII) `A-Z`, `0-9`, space and `x`. The `x` is represented
/// as a lightning glyph in the default LSDJ ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> Name<N> {
    // The special lightning bolt character (the actual glyph depends on your ROM)
    const LIGHTNING_BOLT_CHAR: u8 = 120; // x

    /// Try to convert a byte slice to a name
    ///
    /// This function fails if the bytes are longer than the allowed length, or an invalid
    /// character is found.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        if bytes.len() > N {
            return Err(FromBytesError::TooLong);
        }

        let mut dest = [0; N];
        for (index, byte) in bytes.iter().enumerate() {
            match *byte {
                byte if Self::is_byte_allowed(byte) => dest[index] = byte,
                0 => break,
                _ => return Err(FromBytesError::InvalidByte { byte: *byte, index }),
            }
        }

        Ok(Self { bytes: dest })
    }

    /// Access the underlying bytes that make up the name
    ///
    /// This includes any amount of 0's used for null-termination
    pub fn bytes(&self) -> &[u8; N] {
        &self.bytes
    }

    /// The maximal number of characters allowed in the name
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The number of characters up to the null-termination (or N)
    pub fn len(&self) -> usize {
        self.bytes.iter().position(|c| *c == 0).unwrap_or(N)
    }

    /// Are there _any_ characters in the name string?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to a [`prim@str`] slice
    pub fn as_str(&self) -> &str {
        // SAFETY: Safe, because in from_bytes we check whether any of the characters are within
        // the ASCII subset allowed by LSDJ, which is per definition UTF8-safe.
        unsafe { str::from_utf8_unchecked(&self.bytes[..self.len()]) }
    }

    /// Is a specific byte within the subset of ASCII usable for name strings?
    pub fn is_byte_allowed(byte: u8) -> bool {
        // The only allowed characters are the capitals A-Z, digits 0-9, space or the special
        // lightning bolt character
        (65..=90).contains(&byte) // A-Z
            || (48..=57).contains(&byte) // 0-9
            || byte == 32 // space
            || byte == Self::LIGHTNING_BOLT_CHAR // x
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

impl<'a, const N: usize> TryFrom<&'a [u8]> for Name<N> {
    type Error = FromBytesError;

    #[inline]
    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl<'a, const N: usize> TryFrom<&'a str> for Name<N> {
    type Error = FromBytesError;

    #[inline]
    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        str.as_bytes().try_into()
    }
}

impl<const N: usize> FromStr for Name<N> {
    type Err = FromBytesError;

    #[inline]
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        str.try_into()
    }
}

/// Errors that can result from trying to convert a byte slice to a [`Name`]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FromBytesError {
    /// Error case for when the source slice is too big to fit in the [`Name`] string
    #[error("The slice did not fit in the name array")]
    TooLong,

    /// Only a specific subset of ASCII characters are allowed in [`Name`] strings
    ///
    /// An invalid byte was found during conversion from bytes
    #[error("Byte {byte} at position {index} is not allowed as a name character")]
    InvalidByte { byte: u8, index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes() {
        const HELLO: &str = "HELLO";

        let name = Name::<8>::from_str(HELLO).expect("bytes rejected");
        assert_eq!(name.len(), 5);
        assert!(!name.is_empty());
        assert_eq!(name.as_str(), HELLO);
        assert_eq!(format!("{name}"), HELLO);

        assert_eq!(
            Name::<8>::from_str("123456789"),
            Err(FromBytesError::TooLong)
        );

        assert_eq!(
            Name::<8>::from_str("A!"),
            Err(FromBytesError::InvalidByte {
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
