//! # lsdj-tools
//!
//! [LittleSoundDj](https://www.littlesounddj.com/lsd/index.php), or _LSDJ_ for short, is popular music tracker software for the original [Nintendo Game Boy](https://en.wikipedia.org/wiki/Game_Boy). While the handheld console originally was released in 1989, ([chiptune](https://en.wikipedia.org/wiki/Chiptune)) musicians still use its hardware to create and perform electronic music nowadays.
//!
//! While LSDJ has a built-in filesystem for managing tracks, you need tools to get out the individual files for back-ups or constructing new save files from exported tracks. This crate provides a command-line utility that does exactly that.
//!
//! ## Inspect
//!
//! Inspect LSDJ .sav and .lsdsng files, or even entire directories for their contents
//!
//! ```console
//! USAGE:
//!     lsdj-tools inspect [OPTIONS] [PATH]...
//!
//! ARGS:
//!     <PATH>...    The path(s) to inspect
//!
//! OPTIONS:
//!     -h, --help         Print help information
//!     -r, --recursive    Search the folder recursively
//!     -V, --version      Print version information
//! ```
//!
//! ### Example
//!
//! ```console
//! 4ntler@mbp > lsdj-tools inspect bangers.sav
//! Mem 144/192    [==================      ]
//!   0 | YOKAI    | v027 | f005
//!   1 | ASPHALT  | v019 | f005
//!   2 | NEWSHOES | v014 | f005
//!   3 | FUNGAL   | v019 | f005
//!   4 | LOGCBN   | v015 | f005
//!   5 | NOSTALGA | v031 | f005
//!   6 | GJITSU   | v026 | f005
//!   7 | PRISTINE | v016 | f005
//!   8 | KALEIDO  | v024 | f005
//!   9 | CACTUAR  | v046 | f005
//!  10 | DODGBALL | v018 | f005
//!  11 | DNTSWEAT | v025 | f005
//!  12 | HONEY    | v031 | f005
//! ```
//!
//! ## Export
//!
//! Export .lsdsng's from .sav files
//!
//! ```console
//! USAGE:
//!     lsdj-tools export [OPTIONS] <PATH> [INDEX]...
//!
//! ARGS:
//!     <PATH>        The path to the save file to export from
//!     <INDEX>...    Indices of the songs that should be exported. No indices means all songs
//!
//! OPTIONS:
//!     -d, --decimal            Use decimal version numbers, instead of hexadecimal
//!     -h, --help               Print help information
//!     -o, --output <OUTPUT>    The destination folder to place the songs
//!     -p, --output-pos         Prepend the song position to the start of the filename
//!     -v, --output-version     Append the song version to the end of the filename
//!     -V, --version            Print version information
//! ```
//!
//! ### Example
//!
//! ```console
//! 4ntler@mbp > lsdj-tools export -pv bangers.sav
//! 00. YOKAI    => 00_YOKAI_v1B.lsdsng
//! 01. ASPHALT  => 01_ASPHALT_v13.lsdsng
//! 02. NEWSHOES => 02_NEWSHOES_v0E.lsdsng
//! 03. FUNGAL   => 03_FUNGAL_v13.lsdsng
//! 04. LOGCBN   => 04_LOGCBN_v0F.lsdsng
//! 05. NOSTALGA => 05_NOSTALGA_v1F.lsdsng
//! 06. GJITSU   => 06_GJITSU_v1A.lsdsng
//! 07. PRISTINE => 07_PRISTINE_v10.lsdsng
//! 08. KALEIDO  => 08_KALEIDO_v18.lsdsng
//! 09. CACTUAR  => 09_CACTUAR_v2E.lsdsng
//! 10. DODGBALL => 10_DODGBALL_v12.lsdsng
//! 11. DNTSWEAT => 11_DNTSWEAT_v19.lsdsng
//! 12. HONEY    => 12_HONEY_v1F.lsdsng
//! ```
//!
//! ## Import
//!
//! Import .lsdsng's into a .sav file
//!
//! ```console
//! USAGE:
//!     lsdj-tools import --output <OUTPUT> [SONG]...
//!
//! ARGS:
//!     <SONG>...    Paths to the songs that should be imported into a save
//!
//! OPTIONS:
//!     -h, --help               Print help information
//!     -o, --output <OUTPUT>    The output path
//!     -V, --version            Print version information
//! ```
//!
//! ### Example
//!
//! ```console
//! 4ntler@mbp > lsdj-tools import banger1.lsdsng banger2.lsdsng -o ./test.sav
//! 00 => banger1.lsdsng
//! 01 => banger2.lsdsng
//! Wrote test.sav
//! ```
//!
//! ## Collect
//!
//! Collect goes through a set of files and folders, and matches songs together by their name.
//!
//! It then lists all versions it has found per song, and in which file it has found them.
//!
//! The second column shows a SHA-256 hash of the song contents, to be able to compare them to other versions.
//!
//! Collect is also capable of writing this data to a json file instead.
//!
//! ```console
//! Usage: lsdj-tools collect [OPTIONS] [PATHS]...
//!
//! Arguments:
//!   [PATHS]...
//!           The paths to walk and check for songs
//!
//! Options:
//!   -r, --recursive
//!           Should folders be walked recursively
//!
//!       --json <JSON>
//!           A JSON file the outcome should be written to
//!
//!   -h, --help
//!           Print help (see a summary with '-h')
//!
//!   -V, --version
//!           Print version
//! ```
//!
//! ### Example
//!
//! ```console
//! 4ntler@mbp > lsdj-tools collect ./best_songs_ever
//! BREATHE
//!   v003 86 /usr/best_songs_ever/lsdj9_2_L.sav[0]
//!   v002 43 /usr/best_songs_ever/lsdj9_2_A.sav[0]
//!
//! SUN
//!   v004 f3 /usr/best_songs_ever/lsdj9_4_0_sun.sav[1]
//!   v004 f3 /usr/best_songs_ever/lsdj9_4_0.sav[1]
//! ```

pub mod collect;
pub mod export;
pub mod import;
pub mod inspect;
pub(crate) mod utils;
