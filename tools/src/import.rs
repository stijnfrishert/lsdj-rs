use crate::utils::{has_extension, is_hidden};
use anyhow::{Context, Error, Result};
use clap::Args;
use lsdj::{
    sram::{fs::Filesystem, lsdsng::LsdSng, SRam},
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

pub fn import(args: ImportArgs) -> Result<()> {
    let mut index = 0u8;
    let mut sram = SRam::new();

    for path in args.song {
        if !is_hidden(&path) && has_extension(&path, "lsdsng") {
            if index == Filesystem::FILES_CAPACITY as u8 {
                return Err(Error::msg(
                    "Reached the maximum file limit. Aborting import.",
                ));
            }

            let lsdsng = LsdSng::from_file(&path).context("Could not load {path}")?;
            let song = lsdsng
                .decompress()
                .context(format!("Could not decompress {}", path.to_string_lossy()))?;

            sram.filesystem
                .insert_file(u5::new(index), &lsdsng.name, lsdsng.version, &song)
                .context("Could not insert song")?;

            println!("Imported file {}: {}", index, path.to_string_lossy());

            index += 1;
        }
    }

    sram.to_file(&args.output).context(format!(
        "Could not write SRAM to {}",
        args.output.to_string_lossy()
    ))?;

    println!("Wrote {}", args.output.to_string_lossy());

    Ok(())
}
