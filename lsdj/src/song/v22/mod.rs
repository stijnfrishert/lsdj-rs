use crate::song::SongMemory;
use ux::u4;

/// A V22 song
///
/// Format version 22 has been in use since LSDJ 9.2.1.
pub struct Song {
    /// The wavetable frames
    pub waves: [Wave; 256],
}

impl Song {
    pub fn from_memory(memory: &SongMemory) -> Self {
        assert_eq!(memory.format_version(), 22);

        let waves = [[WaveValue::SILENCE; 32]; 256];

        Song { waves }
    }
}

/// A full wavetable frame of 32 values
pub type Wave = [WaveValue; 32];

/// A single value in a wavetable frame
///
/// Wavetable values are 4-bit integers, where:
///  - 0b0000 means full negative
///  - 0b1000 (8) is equilibrium (no amplitude).
///  - 0b1111 means full positive
///
/// This means the positive side has 1 value less available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaveValue(u4);

impl WaveValue {
    /// The minimum wave value
    pub const MIN: Self = Self(u4::new(0x0));

    /// The wave value representing silence (no amplitude)
    pub const SILENCE: Self = Self(u4::new(0x8));

    /// The maximum wave value
    pub const MAX: Self = Self(u4::new(0xF));
}
