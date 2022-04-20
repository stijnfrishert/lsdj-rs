use std::path::Path;

pub fn is_hidden(path: &Path) -> bool {
    match path.file_name() {
        Some(file_name) => file_name
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false),
        None => false,
    }
}

pub fn has_extension(path: &Path, extension: &str) -> bool {
    match path.extension() {
        Some(ext) => ext == extension,
        None => false,
    }
}
