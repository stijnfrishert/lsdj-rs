use anyhow::Result;
use clap::Parser;

mod export;
mod inspect;

use export::{export, ExportArgs};
use inspect::{inspect, InspectArgs};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
enum Cli {
    Inspect(InspectArgs),
    Export(ExportArgs),
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::Inspect(args) => inspect(&args),
        Cli::Export(args) => export(args),
    }
}
