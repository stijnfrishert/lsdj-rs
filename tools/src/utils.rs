use anyhow::{Context, Result};
use std::{io::stdin, path::Path};
use walkdir::{DirEntry, WalkDir};

pub fn iter_files<'a, I>(
    paths: I,
    recursive: bool,
    extensions: &'a [&'static str],
) -> impl Iterator<Item = DirEntry> + 'a
where
    I: IntoIterator + 'a,
    <I as IntoIterator>::Item: AsRef<Path>,
{
    paths
        .into_iter()
        .flat_map(move |path| {
            let mut walk_dir = WalkDir::new(path.as_ref());
            if !recursive {
                walk_dir = walk_dir.max_depth(1);
            }

            walk_dir
        })
        .filter_map(Result::ok)
        .filter(|entry| {
            !is_hidden(entry)
                && extensions
                    .iter()
                    .any(|extension| *extension == entry.path().extension().unwrap_or_default())
        })
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

pub fn has_extension(path: &Path, extension: &str) -> bool {
    match path.extension() {
        Some(ext) => ext == extension,
        None => false,
    }
}

pub fn check_for_overwrite(path: &Path) -> Result<()> {
    if path.exists() {
        loop {
            println!(
                "{} already exists. Do you want to overwrite it? Y/n",
                path.to_string_lossy()
            );

            let mut line = String::new();
            stdin()
                .read_line(&mut line)
                .context("Could not read terminal input")?;

            match line.as_str() {
                "Y\n" => break,
                "n\n" => std::process::exit(0),
                _ => (),
            }
        }
    }

    Ok(())
}
