use std::fs;
use std::io::Read;
use std::path::PathBuf;
use walkdir::WalkDir;
use crate::language::{LanguageRegistry, LanguageSpec};

pub struct FileEntry {
    pub path: PathBuf,
    pub language: &'static LanguageSpec,
}

pub fn walk_files(paths: &[PathBuf], registry: &'static LanguageRegistry) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for path in paths {
        if path.is_file() {
            if let Some(lang) = detect(path, registry) {
                entries.push(FileEntry { path: path.clone(), language: lang });
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| {
                    e.depth() == 0 || !e.file_name().to_str().is_some_and(|s| s.starts_with('.'))
                })
                .flatten()
            {
                if entry.file_type().is_file() {
                    if let Some(lang) = detect(entry.path(), registry) {
                        entries.push(FileEntry {
                            path: entry.into_path(),
                            language: lang,
                        });
                    }
                }
            }
        }
    }
    entries
}

fn detect(
    path: &std::path::Path,
    registry: &'static LanguageRegistry,
) -> Option<&'static LanguageSpec> {
    if let Some(lang) = registry.detect_by_ext(path) {
        return Some(lang);
    }
    let mut buf = [0u8; 256];
    if let Ok(mut f) = fs::File::open(path) {
        if let Ok(n) = f.read(&mut buf) {
            let first_line_end = buf[..n].iter().position(|&b| b == b'\n').unwrap_or(n);
            return registry.detect_by_shebang(&buf[..first_line_end]);
        }
    }
    None
}
