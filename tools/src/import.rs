use crate::utils::{has_extension, is_hidden};
use anyhow::{Context, Result};
use clap::Args;
use lsdj::sram::{lsdsng::LsdSng, SRam};
use std::path::PathBuf;

/// Import songs into an LSDJ save file
#[derive(Args)]
#[clap(author, version)]
pub struct ImportArgs {
    /// Paths to the songs that shoud be imported into a save
    song: Vec<PathBuf>,

    /// The output path (or a default name if not provided)
    output: Option<PathBuf>,
}

pub fn import(args: ImportArgs) -> Result<()> {
    let mut songs = Vec::new();

    for path in args.song {
        if !is_hidden(&path) && has_extension(&path, "lsdsng") {
            songs.push(LsdSng::from_file(&path).context("Could not load {path}")?);
        }
    }

    let _sram = SRam::new();

    Ok(())
}
