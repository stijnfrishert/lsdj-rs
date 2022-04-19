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

    Ok(())
}
