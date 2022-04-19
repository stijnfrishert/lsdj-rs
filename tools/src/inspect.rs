use anyhow::{Context, Result};
use clap::Args;
use lsdj::sram::{fs::Filesystem, SRam};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Args)]
pub struct InspectArgs {
    /// The path to the file to inspect
    path: PathBuf,
}

pub fn inspect(args: &InspectArgs) -> Result<()> {
    let paths: Vec<_> = WalkDir::new(&args.path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(get_path_if_valid)
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
    let sram = SRam::from_file(&path).context("Reading the SRAM from file failed")?;

    println!(
        "{:<32}Mem {}/{}",
        path.file_name().unwrap().to_string_lossy(),
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

fn get_path_if_valid(entry: DirEntry) -> Option<PathBuf> {
    if !is_hidden(&entry) {
        let path = entry.path();
        if has_supported_extension(path) {
            return Some(path.to_owned());
        }
    }

    None
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn has_supported_extension(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => ext == "sav",
        None => false,
    }
}
