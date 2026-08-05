use crate::language::{LanguageRegistry, LanguageSpec};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: PathBuf,
    pub language: &'static LanguageSpec,
}

/// Directories ignored by default: build-cache and dependency directories.
/// Users can add or remove patterns, or disable the whole set.
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

/// Directory names to skip while walking. Patterns match one name (not a
/// path): "dist" ignores any `dist/` at any depth.
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
                entries.push(FileEntry {
                    path: path.clone(),
                    language: lang,
                });
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| {
                    if e.depth() == 0 {
                        return true;
                    }
                    let name = e.file_name().to_str().unwrap_or("");
                    // Skip hidden dot-dirs and explicitly ignored dirs.
                    !(name.starts_with('.') || ignore.is_ignored(name))
                })
                .flatten()
            {
                if entry.file_type().is_file()
                    && let Some(lang) = detect(entry.path(), registry)
                {
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
        && let Ok(n) = f.read(&mut buf)
    {
        let end = buf[..n].iter().position(|&b| b == b'\n').unwrap_or(n);
        Some(&buf[..end])
    } else {
        None
    };
    registry.detect(path, first_line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::registry;

    #[test]
    fn walk_skips_hidden_and_ignored_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("target")).unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        std::fs::write(dir.path().join("src/a.rs"), b"fn a() {}").unwrap();
        std::fs::write(dir.path().join("target/b.rs"), b"fn b() {}").unwrap();
        std::fs::write(dir.path().join(".hidden/c.rs"), b"fn c() {}").unwrap();
        let ignore = DirIgnore::new(true);
        let entries = walk_files(&[dir.path().to_path_buf()], registry(), &ignore);
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a.rs"], "must skip target/ and hidden dirs");
    }

    #[test]
    fn walk_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.rs");
        std::fs::write(&f, b"fn x() {}").unwrap();
        let ignore = DirIgnore::new(false);
        let entries = walk_files(std::slice::from_ref(&f), registry(), &ignore);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].language.name, "Rust");
    }

    #[test]
    fn walk_skips_unrecognized_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.zzunknown"), b"x").unwrap();
        let ignore = DirIgnore::new(false);
        let entries = walk_files(&[dir.path().to_path_buf()], registry(), &ignore);
        assert!(entries.is_empty());
    }

    #[test]
    fn dir_ignore_add_remove() {
        let mut ig = DirIgnore::new(false);
        assert!(!ig.is_ignored("dist"));
        ig.add("dist");
        assert!(ig.is_ignored("dist"));
        ig.remove("dist");
        assert!(!ig.is_ignored("dist"));
    }

    #[test]
    fn dir_ignore_defaults() {
        let ig = DirIgnore::new(true);
        assert!(ig.is_ignored("target"));
        assert!(ig.is_ignored("node_modules"));
        assert!(!ig.is_ignored("src"));
        let none = DirIgnore::new(false);
        assert!(!none.is_ignored("target"));
    }
}
