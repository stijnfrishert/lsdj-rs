use anyhow::Result;
use clap::Parser;

use lsdj_tools::collect::{CollectArgs, collect};
use lsdj_tools::export::{ExportArgs, export};
use lsdj_tools::import::{ImportArgs, import};
use lsdj_tools::inspect::{InspectArgs, inspect};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
enum Cli {
    Inspect(InspectArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Collect(CollectArgs),
}

fn main() -> Result<()> {
    match Cli::parse_from(wild::args()) {
        Cli::Inspect(args) => inspect(&args),
        Cli::Export(args) => export(args),
        Cli::Import(args) => import(args),
        Cli::Collect(args) => collect(args),
    }
}
