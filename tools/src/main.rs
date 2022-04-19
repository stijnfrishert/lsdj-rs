use anyhow::{Context, Result};
use clap::Parser;
use lsdj::sram::SRam;
use std::fs::File;

#[derive(Parser)]
enum Cli {
    List { path: String },
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List { path } => list(&path),
    }
}

fn list(path: &str) -> Result<()> {
    let file = File::open(path).context("Opening {path} failed")?;
    let sram = SRam::from_reader(file).context("Parsing the SRAM failed")?;

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
