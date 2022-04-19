use anyhow::{Context, Result};
use clap::Parser;
use lsdj::sram::SRam;

#[derive(Parser)]
enum Cli {
    List { path: String },
    Export { path: String },
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List { path } => list(&path),
        Cli::Export { path } => export(&path),
    }
}

fn list(path: &str) -> Result<()> {
    let sram = SRam::from_file(path).context("Reading the SRAM from file failed")?;

    for (index, file) in sram.filesystem.files().enumerate() {
        if let Some(file) = file {
            let song = file.decompress().context("Could not decompress file")?;

            println!(
                "{index:>2} | {:<8} | v{:02X} | f{:02X}",
                format!("{}", file.name().context("Could not parse the file name")?),
                file.version(),
                song.format_version()
            );
        }
    }

    Ok(())
}

fn export(path: &str) -> Result<()> {
    let sram = SRam::from_file(path).context("Reading the SRAM from file failed")?;

    for (_index, file) in sram.filesystem.files().enumerate() {
        if let Some(_file) = file {
            //
        }
    }

    Ok(())
}
