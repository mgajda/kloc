use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::collections::HashSet;
use walkdir::WalkDir;
use crate::language::{LanguageRegistry, LanguageSpec};

pub struct FileEntry {
    pub path: PathBuf,
    pub language: &'static LanguageSpec,
}

/// Directories ignored by default: build-cache / dependency directories that
/// are not project source code. Users can add or remove patterns, or disable
/// the whole default set.
const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".vscode",
    ".opencode",
    ".claude",
    ".cache",
    "__pycache__",
    "dist",
    "dist-newstyle",
    ".stack",
    ".cabal",
    "target",
];

/// Which directory names to skip while walking. Patterns match a single
/// directory name (not a path), so a pattern like "dist" ignores any `dist/`
/// directory at any depth.
#[derive(Debug, Clone, Default)]
pub struct DirIgnore {
    /// Names to skip. Start from the defaults unless `defaults` is false.
    names: HashSet<String>,
}

impl DirIgnore {
    pub fn new(defaults: bool) -> Self {
        let names = if defaults {
            DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect()
        } else {
            HashSet::new()
        };
        DirIgnore { names }
    }

    pub fn add(&mut self, name: &str) {
        self.names.insert(name.to_string());
    }

    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }

    pub fn is_ignored(&self, dir_name: &str) -> bool {
        self.names.contains(dir_name)
    }
}

pub fn walk_files(
    paths: &[PathBuf],
    registry: &'static LanguageRegistry,
    ignore: &DirIgnore,
) -> Vec<FileEntry> {
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
                    if e.depth() == 0 { return true; }
                    let name = e.file_name().to_str().unwrap_or("");
                    // Skip hidden dot-dirs and explicitly ignored dirs.
                    !(name.starts_with('.') || ignore.is_ignored(name))
                })
                .flatten()
            {
                if entry.file_type().is_file()
                    && let Some(lang) = detect(entry.path(), registry) {
                        entries.push(FileEntry {
                            path: entry.into_path(),
                            language: lang,
                        });
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
    let mut buf = [0u8; 256];
    let first_line = if let Ok(mut f) = fs::File::open(path)
        && let Ok(n) = f.read(&mut buf) {
            let end = buf[..n].iter().position(|&b| b == b'\n').unwrap_or(n);
            Some(&buf[..end])
        } else {
            None
        };
    registry.detect(path, first_line)
}
