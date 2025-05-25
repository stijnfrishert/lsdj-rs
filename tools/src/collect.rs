//! The `collect` subcommand

use crate::utils::iter_files;
use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use lsdj::{fs::File, lsdsng::LsdSng, sram::SRam};
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

/// Arguments for the `collect` subcommand
#[derive(Args)]
#[clap(
    author,
    version,
    about = "Collect all versions of a (set of) song and print them",
    long_about = "Collect goes through a set of files and folders, and matches songs together by their name.\n\nIt then lists all versions it has found per song, and in which file it has found them.\n\nThe second column shows a SHA-256 hash of the song contents, to be able to compare them to other versions.\n\nCollect is also capable of writing this data to a json file instead."
)]
pub struct CollectArgs {
    /// The paths to walk and check for songs
    paths: Vec<PathBuf>,

    /// Should folders be walked recursively
    #[clap(short, long)]
    recursive: bool,

    /// A JSON file the outcome should be written to
    #[clap(long)]
    json: Option<PathBuf>,
}

/// Collect all versions of a (set of) song and print them
pub fn collect(args: CollectArgs) -> Result<()> {
    if args.paths.is_empty() {
        println!("No paths provided to collect from");
        return Ok(());
    }

    // Collect the songs
    let outcome = collect_songs(args.paths, args.recursive);

    // Go over the songs and print the songs we found
    if let Some(path) = args.json {
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)
            .context(format!("Could not create folder at {}", parent.display()))?;

        let file = fs::File::create(&path)
            .context(format!("Could not create file at {}", path.display()))?;

        serde_json::to_writer_pretty(file, &outcome).context("Could not write to JSON")?;

        println!("Wrote to {}", path.display());
    } else {
        print_outcome(outcome);
    }

    Ok(())
}

fn collect_songs(paths: Vec<PathBuf>, recursive: bool) -> Outcome {
    let mut outcome = Outcome::default();

    // Collect the instances
    for entry in iter_files(paths, recursive, &["sav"]) {
        let path = entry.path();

        if let Some(extension) = path.extension() {
            if extension == "sav" {
                if let Ok(sram) = SRam::from_path(path) {
                    for (index, entry) in sram.filesystem.files().enumerate() {
                        if let Some(entry) = entry {
                            let name = entry.name().unwrap().as_str().to_owned();
                            let source = Source::Sav {
                                path: path.to_owned(),
                                index,
                            };
                            match file_to_instance(&entry, source.clone()) {
                                Some(instance) => {
                                    outcome.songs.entry(name).or_default().push(instance);
                                }
                                None => outcome.errors.push(source),
                            }
                        }
                    }
                }
            } else if extension == "lsdsng" {
                if let Ok(lsdsng) = LsdSng::from_path(path) {
                    let name = lsdsng.name().unwrap().as_str().to_owned();
                    let source = Source::LsdSng {
                        path: path.to_owned(),
                    };

                    match file_to_instance(&lsdsng, source.clone()) {
                        Some(instance) => {
                            outcome.songs.entry(name).or_default().push(instance);
                        }
                        None => outcome.errors.push(source),
                    }
                }
            }
        }
    }

    outcome
}

fn file_to_instance(file: &impl File, source: Source) -> Option<Instance> {
    let version = file.version();
    let song = file.decompress().ok()?;
    let sha = Sha256::digest(song.as_slice());

    Some(Instance {
        version,
        sha256: sha.into(),
        source,
    })
}

fn print_outcome(outcome: Outcome) {
    let mut first = true;

    for source in outcome.errors {
        match source {
            Source::LsdSng { path } => {
                println!("Could not decompress {}", path.display());
            }
            Source::Sav { path, index } => {
                println!(
                    "Could not decompress {}[{}]",
                    path.display(),
                    index.to_string().blue()
                );
            }
        }
    }

    for (name, mut instances) in outcome.songs {
        if first {
            first = false;
        } else {
            println!();
        }

        println!("{}", name.bold());

        instances.sort_by_key(|instance| instance.version);
        instances.reverse();

        // Collect the hash strings
        let shas = instances
            .iter()
            .map(|instance| &instance.sha256)
            .collect::<HashSet<_>>();
        let unique_sha_length = find_min_len(shas);

        for instance in instances {
            let version = format!("v{:03}", instance.version).green();
            let sha = bytes_to_string(&instance.sha256);
            let sha = sha[..unique_sha_length * 2].dimmed();

            match instance.source {
                Source::LsdSng { path } => {
                    println!("  v{version} {sha} {}", path.display());
                }
                Source::Sav { path, index } => {
                    println!(
                        "  {version} {sha} {}[{}]",
                        path.display(),
                        index.to_string().blue()
                    );
                }
            }
        }
    }
}

fn bytes_to_string(sha: &[u8; 32]) -> String {
    sha.iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

/// The minimum amount of bytes needed to uniquely identify each byte string in a set
fn find_min_len(strings: HashSet<&[u8; 32]>) -> usize {
    let mut unique_length = 0;
    let mut seen = HashSet::new();

    for i in 0..32 {
        for string in &strings {
            let prefix = &string[..=i];
            if seen.insert(prefix) {
                unique_length = i + 1;
            }
        }
        if unique_length == i + 1 {
            break;
        }
    }

    unique_length
}

#[derive(Default, Serialize)]
struct Outcome {
    pub songs: HashMap<String, Vec<Instance>>,
    pub errors: Vec<Source>,
}

#[derive(Serialize)]
struct Instance {
    version: u8,

    #[serde(serialize_with = "sha_serialize")]
    sha256: [u8; 32],
    source: Source,
}

fn sha_serialize<S>(x: &[u8; 32], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&bytes_to_string(x))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum Source {
    LsdSng { path: PathBuf },
    Sav { path: PathBuf, index: usize },
}
