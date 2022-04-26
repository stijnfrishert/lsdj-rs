use crate::utils::check_for_overwrite;
use anyhow::{Context, Result};
use clap::Args;
use lsdj::sram::{
    file::{filesystem::Filesystem, File},
    SRam,
};
use std::{env::current_dir, fs::create_dir_all};

use std::path::PathBuf;

/// Export songs from an LSDJ save file
#[derive(Args)]
#[clap(author, version)]
pub struct ExportArgs {
    /// The path to the save file to export from
    path: PathBuf,

    /// Indices of the songs that should be exported. No indices means all songs.
    index: Vec<usize>,

    /// The destination folder to place the songs
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// Prepend the song position to the start of the filename
    #[clap(short = 'p', long)]
    output_pos: bool,

    /// Append the song version to the end of the filename
    #[clap(short = 'v', long)]
    output_version: bool,

    /// Use decimal version numbers, instead of hexadecimal
    #[clap(short, long)]
    decimal: bool,
}

pub fn export(mut args: ExportArgs) -> Result<()> {
    let sram = SRam::from_path(&args.path).context("Reading the SRAM from file failed")?;

    if args.index.is_empty() {
        args.index = (0..Filesystem::FILES_CAPACITY).collect();
    }

    let folder = match args.output {
        Some(folder) => folder,
        None => current_dir().context("Could not fetch current working directory")?,
    };
    create_dir_all(&folder).context("Could not create output directory")?;

    for (index, file) in sram.filesystem.files().enumerate() {
        if !args.index.contains(&index) {
            continue;
        }

        if let Some(file) = file {
            let lsdsng = file
                .lsdsng()
                .context("Could not create an LsdSng from an SRAM file slot")?;

            let mut filename = String::new();
            if args.output_pos {
                filename.push_str(&format!("{:02}_", index));
            }

            filename.push_str(lsdsng.name.as_str());
            if args.output_version {
                if args.decimal {
                    filename.push_str(&format!("_v{:03}", lsdsng.version));
                } else {
                    filename.push_str(&format!("_v{:02X}", lsdsng.version));
                }
            }

            let path = folder.join(filename).with_extension("lsdsng");

            check_for_overwrite(&path)?;

            lsdsng
                .to_path(&path)
                .context("Could not write lsdsng to file")?;

            println!(
                "{:02}. {:8} => {}",
                index,
                lsdsng.name.as_str(),
                path.to_string_lossy()
            );
        }
    }

    Ok(())
}
