use anyhow::{Context, Result};
use clap::Args;
use lsdj::sram::{fs::Filesystem, SRam};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Inspect LSDJ save files for their contents
#[derive(Args)]
#[clap(author, version)]
pub struct InspectArgs {
    /// The path to the file to inspect
    path: PathBuf,

    /// Search the folder recursively
    #[clap(short, long)]
    recursive: bool,
}

pub fn inspect(args: &InspectArgs) -> Result<()> {
    let mut walk_dir = WalkDir::new(&args.path);
    if !args.recursive {
        walk_dir = walk_dir.max_depth(1);
    }

    let paths: Vec<_> = walk_dir
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(get_path_if_valid)
        .collect();

    if let Some((last, rest)) = paths.split_last() {
        for path in rest {
            print(path, args)?;
            println!();
        }

        print(last, args)?;
    }

    Ok(())
}

fn print(path: &Path, args: &InspectArgs) -> Result<()> {
    let sram = SRam::from_file(&path).context("Reading the SRAM from file failed")?;

    let path = diff_paths(path, &args.path).unwrap();

    println!("{}", path.to_string_lossy());
    print_mem(&sram);

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

fn print_mem(sram: &SRam) {
    const BAR_LEN: usize = 24;
    let blocks = sram.filesystem.blocks_used_count();
    let bar = blocks * BAR_LEN / Filesystem::BLOCKS_CAPACITY;

    println!(
        "Mem {:03}/{:03}    [{}{}]",
        blocks,
        Filesystem::BLOCKS_CAPACITY,
        "=".repeat(bar),
        " ".repeat(BAR_LEN - bar)
    );
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
