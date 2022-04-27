//! # LSDJ
//!
//! [LittleSoundDj](https://www.littlesounddj.com/lsd/index.php), or _LSDJ_ for short, is popular music tracker software for the original [Nintendo Game Boy](https://en.wikipedia.org/wiki/Game_Boy). While the handheld console originally was released in 1989, ([chiptune](https://en.wikipedia.org/wiki/Chiptune)) musicians still use its hardware to create and perform electronic music nowadays.
//!
//! While LSDJ has a built-in filesystem for managing tracks, you need tools to get out the individual files for back-ups or constructing new save files from exported tracks. The developer behind LSDJ has also made [LSDPatcher](https://github.com/jkotlinski/lsdpatch), a GUI package for managing your songs.
//!
//! This crate provides an alternative library for manipulating LSDJ save files/SRAM, in combination with a command-line utility for managing your songs. It was inspired by my work on [liblsdj](https://github.com/stijnfrishert/liblsdj), an equivalent library in C.
//!
//! ## Example
//!
//! ```rust no_run
//! use lsdj::{
//!     sram::SRam,
//!     fs::{Index, File}
//! };
//!
//! // Load a save file from disk
//! let sram = SRam::from_path("bangers.sav").expect("Could not load SRAM");
//!
//! // Access one of the files
//! if let Some(file) = sram.filesystem.file(Index::new(0)) {
//!     // Convert the file to an .lsdsng (common song format)
//!     let lsdsng = file.lsdsng().expect("Could not convert file to LsdSng");
//!
//!     // Store the song on disk
//!     lsdsng.to_path("put_yo_hands_up.lsdsng").expect("Could not save LsdSng");
//! }
//! ```
//!
//! ## Features
//!
//! The crate currently supports the following functionality:
//!
//! - [`SRAM`](crate::sram) serialization and deserialization
//! - [`Filesystem`](crate::fs) manipulation (querying, inserting and removing files)
//! - [`LsdSng`](crate::lsdsng) serialization and deserialization
//! - Full implementation of the [compression algorithm](crate::serde)
//!
//! ## Wishlist
//!
//! These are features I'm interested in exploring/adding at a certain point:
//!
//! - [`SongMemory`](crate::song) parsing into song structures per format version. (This would allow manipulating songs.)
//! - `.lsdprj` support
//! - `ROM` handling, mainly for sample manipulation
//!
//! ## Support
//!
//! If you like this crate and want to support me somehow, consider buying some of [my music](https://4ntler.bandcamp.com/).

pub mod fs;
pub mod lsdsng;
pub mod name;
pub mod serde;
pub mod song;
pub mod sram;
