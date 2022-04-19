use anyhow::{Context, Result};
use clap::Parser;
use lsdj::sram::{fs::Filesystem, SRam};
use std::{
    env::current_dir,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

#[derive(Parser)]
enum Cli {
    List {
        path: String,
    },
    Export {
        path: String,

        /// Indices of the songs that should be exported. No indices means all songs.
        #[clap(short, long)]
        index: Vec<usize>,

        /// The destination folder to place the songs
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List { path } => list(&path),
        Cli::Export {
            path,
            index,
            output,
        } => export(&path, index, output.as_deref()),
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

fn export(path: &str, mut indices: Vec<usize>, output: Option<&Path>) -> Result<()> {
    let path = Path::new(path);
    let sram = SRam::from_file(path).context("Reading the SRAM from file failed")?;

    if indices.is_empty() {
        indices = (0..Filesystem::FILES_CAPACITY).collect();
    }

    let folder = match output {
        Some(folder) => folder.to_owned(),
        None => current_dir().context("Could not fetch current working directory")?,
    };
    create_dir_all(&folder).context("Could not create output directory")?;

    for (index, file) in sram.filesystem.files().enumerate() {
        if !indices.contains(&index) {
            continue;
        }

        if let Some(file) = file {
            let lsdsng = file
                .lsdsng()
                .context("Could not create an LsdSng from an SRAM file slot")?;

            let path = folder.join(format!(
                "{:02X}. {} v{:02X}.lsdsng",
                index,
                lsdsng.name.as_str(),
                lsdsng.version
            ));

            lsdsng
                .to_file(path)
                .context("Could not write lsdsng to file")?;
        }
    }

    Ok(())
}
