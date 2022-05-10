//! The `render` subcommand

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

/// Arguments for the `render` subcommand
#[derive(Args)]
#[clap(author, version, about = "Render a song to an audio file", long_about = None)]
pub struct RenderArgs {
    /// The path to the ROM to use
    #[clap(short, long)]
    rom: PathBuf,

    /// The path to the ROM to use
    lsdsng: PathBuf,
}

/// Render LSDJ .sav and .lsdsng files, or even entire directories for their contents
pub fn render(args: RenderArgs) -> Result<()> {
    println!("Rendering with ROM {}", args.rom.to_string_lossy());
    println!("Rendering {}", args.lsdsng.to_string_lossy());
    Ok(())
}
