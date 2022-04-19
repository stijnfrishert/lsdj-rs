use anyhow::{Context, Result};
use clap::Args;
use lsdj::sram::{fs::Filesystem, SRam};
use std::path::PathBuf;

#[derive(Args)]
pub struct InspectArgs {
    /// The path to the file to inspect
    path: PathBuf,
}

pub fn inspect(args: &InspectArgs) -> Result<()> {
    let sram = SRam::from_file(&args.path).context("Reading the SRAM from file failed")?;

    println!(
        "{:<32}Mem {}/{}",
        args.path.file_name().unwrap().to_string_lossy(),
        sram.filesystem.blocks_used_count(),
        Filesystem::BLOCKS_CAPACITY
    );

    for (index, file) in sram.filesystem.files().enumerate() {
        if let Some(file) = file {
            let song = file.decompress().context("Could not decompress file")?;

            println!(
                "{index:>3} | {:<8} | v{:03} | f{:03}",
                format!("{}", file.name().context("Could not parse the file name")?),
                file.version(),
                song.format_version()
            );
        }
    }

    Ok(())
}
