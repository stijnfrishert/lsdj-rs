use anyhow::Result;
use clap::Parser;

use lsdj_tools::export::{export, ExportArgs};
use lsdj_tools::import::{import, ImportArgs};
use lsdj_tools::inspect::{inspect, InspectArgs};
use lsdj_tools::render::{render, RenderArgs};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
enum Cli {
    Inspect(InspectArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Render(RenderArgs),
}

fn main() -> Result<()> {
    match Cli::parse_from(wild::args()) {
        Cli::Inspect(args) => inspect(&args),
        Cli::Export(args) => export(args),
        Cli::Import(args) => import(args),
        Cli::Render(args) => render(args),
    }
}
