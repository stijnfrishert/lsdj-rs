use anyhow::Result;
use clap::Parser;

use lsdj_tools::export::{export, ExportArgs};
use lsdj_tools::inspect::{inspect, InspectArgs};

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
