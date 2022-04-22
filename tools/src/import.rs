use crate::utils::{has_extension, is_hidden};
use anyhow::{Context, Result};
use clap::Args;
use lsdj::{
    sram::{lsdsng::LsdSng, SRam},
    u5,
};
use std::path::PathBuf;

/// Import songs into an LSDJ save file
#[derive(Args)]
#[clap(author, version)]
pub struct ImportArgs {
    /// Paths to the songs that shoud be imported into a save
    song: Vec<PathBuf>,

    /// The output path (or a default name if not provided)
    #[clap(short, long)]
    output: PathBuf,
}

// struct Song {
//     name: Name<8>,
//     version: u8,
//     song: SongMemory,
// }

pub fn import(args: ImportArgs) -> Result<()> {
    let mut songs = Vec::new();

    for path in args.song {
        if !is_hidden(&path) && has_extension(&path, "lsdsng") {
            songs.push(LsdSng::from_file(&path).context("Could not load {path}")?);
        }
    }

    let mut sram = SRam::new();

    sram.filesystem
        .insert_file(
            u5::new(0),
            &songs[0].name,
            songs[0].version,
            &songs[0].decompress().context("Could not decompress song")?,
        )
        .context("Could not insert song")?;

    sram.to_file(&args.output)
        .context("Could not write SRAM to file")?;
    println!("Wrote {}", args.output.to_string_lossy());

    Ok(())
}
