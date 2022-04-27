//! The `import` subcommand

use crate::utils::{check_for_overwrite, has_extension, iter_files};
use anyhow::{Context, Error, Result};
use clap::Args;
use lsdj::{
    fs::{File, Filesystem, Index},
    lsdsng::LsdSng,
    name::Name,
    serde::CompressBlockError,
    song::SongMemory,
    sram::SRam,
};
use std::path::PathBuf;

/// Arguments for the `import` subcommand
#[derive(Args)]
#[clap(author, version, about = "Import .lsdsng's into a .sav file", long_about = None)]
pub struct ImportArgs {
    /// Paths to the songs that should be imported into a save
    song: Vec<PathBuf>,

    /// The output path
    #[clap(short, long)]
    output: PathBuf,
}

/// Import .lsdsng's into a .sav file
pub fn import(args: ImportArgs) -> Result<()> {
    let mut index = 0u8;
    let mut sram = SRam::new();

    for entry in iter_files(&args.song, true, &["lsdsng", "sav"]) {
        let path = entry.path();

        if index == Filesystem::FILES_CAPACITY as u8 {
            return Err(Error::msg(
                "Reached the maximum file limit. Aborting import.",
            ));
        }

        if has_extension(path, "lsdsng") {
            let lsdsng = LsdSng::from_path(&path).context("Could not load {path}")?;
            let song = lsdsng
                .decompress()
                .context(format!("Could not decompress {}", path.to_string_lossy()))?;

            insert(&mut sram, index, &lsdsng.name, lsdsng.version, &song)?;

            println!("{:02} => {}", index, path.to_string_lossy());

            index += 1;
        } else if has_extension(path, "sav") {
            let sav = SRam::from_path(&path)
                .context(format!("Could not open {}", path.to_string_lossy()))?;

            for (source_index, file) in sav.filesystem.files().enumerate() {
                if let Some(file) = file {
                    let song = file.decompress().context(format!(
                        "Could not decompress file {} from {}",
                        source_index,
                        path.to_string_lossy()
                    ))?;

                    let name = file.name()?;

                    insert(&mut sram, index, &name, file.version(), &song)?;

                    println!(
                        "{:02} => {} - {}",
                        index,
                        path.to_string_lossy(),
                        name.as_str(),
                    );

                    index += 1;
                }
            }
        }
    }

    if check_for_overwrite(&args.output)? {
        sram.to_path(&args.output).context(format!(
            "Could not write SRAM to {}",
            args.output.to_string_lossy()
        ))?;

        println!("Wrote {}", args.output.to_string_lossy());
    }

    Ok(())
}

fn insert(
    sram: &mut SRam,
    index: u8,
    name: &Name<8>,
    version: u8,
    song: &SongMemory,
) -> Result<()> {
    match sram
        .filesystem
        .insert_file(Index::new(index), name, version, song)
    {
        Err(CompressBlockError::NoBlockLeft) => {
            Err(Error::msg("Ran out of space in the SRAM memory"))
        }
        result => {
            result.context("Could not insert song")?;
            Ok(())
        }
    }
}
