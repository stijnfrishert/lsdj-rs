//! The `inspect` subcommand

use crate::utils::iter_files;
use anyhow::{Context, Result};
use clap::Args;
use lsdj::{
    fs::{File, Filesystem},
    lsdsng::LsdSng,
    sram::SRam,
};
use std::path::{Path, PathBuf};

/// Arguments for the `inspect` subcommand
#[derive(Args)]
#[clap(author, version, about = "Inspect LSDJ .sav and .lsdsng files, or even entire directories for their contents", long_about = None)]
pub struct InspectArgs {
    /// The path(s) to inspect
    path: Vec<PathBuf>,

    /// Search the folder recursively
    #[clap(short, long)]
    recursive: bool,
}

/// Inspect LSDJ .sav and .lsdsng files, or even entire directories for their contents
pub fn inspect(args: &InspectArgs) -> Result<()> {
    let paths: Vec<_> = iter_files(&args.path, args.recursive, &["sav", "lsdsng"])
        .map(|entry| entry.path().to_owned())
        .collect();

    if let Some((last, rest)) = paths.split_last() {
        for path in rest {
            print(path)?;
            println!();
        }

        print(last)?;
    }

    Ok(())
}

fn print(path: &Path) -> Result<()> {
    println!("{}", path.to_string_lossy());

    match path.extension().and_then(|str| str.to_str()) {
        Some("sav") => {
            let sram = SRam::from_path(path).context("Reading the SRAM from file failed")?;

            print_mem(&sram);

            for (index, file) in sram.filesystem.files().enumerate() {
                if let Some(file) = file {
                    print_file(index, &file)?;
                }
            }
        }
        Some("lsdsng") => {
            let lsdsng = LsdSng::from_path(path).context("Reading the LsdSng from file failed")?;
            print_file(0, &lsdsng)?;
        }
        _ => (),
    }

    Ok(())
}

fn print_mem(sram: &SRam) {
    const BAR_LEN: usize = 24;
    let blocks = sram.filesystem.blocks_used_count();
    let bar = blocks * BAR_LEN / Filesystem::BLOCKS_CAPACITY;

    println!(
        "Mem {:03}/{:03}    [{}{}]",
        blocks,
        Filesystem::BLOCKS_CAPACITY,
        "=".repeat(bar),
        " ".repeat(BAR_LEN - bar)
    );
}

fn print_file(index: usize, file: &impl File) -> Result<()> {
    let song = file.decompress().context("Could not decompress file")?;

    println!(
        "{index:>3} | {:<8} | v{:03} | f{:03}",
        format!("{}", file.name().context("Could not parse the file name")?),
        file.version(),
        song.format_version()
    );

    Ok(())
}
