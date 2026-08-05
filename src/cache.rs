//! On-disk result cache keyed by (canonical path, size, mtime).
//! Disabled with `--no-cache`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::complexity::ComplexityResult;
use crate::counter::CountResult;

pub struct Cache {
    enabled: bool,
    dir: Option<PathBuf>,
    stats: Mutex<CacheStats>,
}

#[derive(Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl Cache {
    pub fn new(enabled: bool) -> Self {
        let dir = if enabled {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
                .map(|base| base.join("kloc"))
        } else {
            None
        };
        if let Some(ref d) = dir {
            let _ = std::fs::create_dir_all(d);
        }
        Cache {
            enabled,
            dir,
            stats: Mutex::new(CacheStats::default()),
        }
    }

    fn entry_path(&self, path: &Path) -> Option<PathBuf> {
        let dir = self.dir.as_ref()?;
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = canonical.to_string_lossy();
        let hash = fxhash64(key.as_bytes());
        Some(dir.join(format!("{hash:016x}.json")))
    }

    pub fn get(
        &self,
        path: &Path,
        size: u64,
        mtime_ns: u64,
    ) -> Option<(CountResult, ComplexityResult)> {
        if !self.enabled {
            return None;
        }
        let ep = match self.entry_path(path) {
            Some(p) => p,
            None => {
                self.bump_miss();
                return None;
            }
        };
        let bytes = match std::fs::read(&ep) {
            Ok(b) => b,
            Err(_) => {
                self.bump_miss();
                return None;
            }
        };
        let entry: CachedEntry = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => {
                self.bump_miss();
                return None;
            }
        };
        if entry.size == size && entry.mtime_ns == mtime_ns {
            let mut s = self.stats.lock().unwrap();
            s.hits += 1;
            Some((entry.count, entry.complexity))
        } else {
            self.bump_miss();
            None
        }
    }

    fn bump_miss(&self) {
        let mut s = self.stats.lock().unwrap();
        s.misses += 1;
    }

    pub fn put(
        &self,
        path: &Path,
        size: u64,
        mtime_ns: u64,
        count: &CountResult,
        complexity: &ComplexityResult,
    ) {
        if !self.enabled {
            return;
        }
        let Some(ep) = self.entry_path(path) else {
            return;
        };
        let entry = CachedEntry {
            size,
            mtime_ns,
            count: *count,
            complexity: complexity.clone(),
        };
        if let Ok(bytes) = serde_json::to_vec(&entry) {
            let _ = std::fs::write(ep, bytes);
        }
    }

    pub fn stats(&self) -> (u64, u64) {
        let s = self.stats.lock().unwrap();
        (s.hits, s.misses)
    }

    #[cfg(test)]
    pub(crate) fn with_dir(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Cache {
            enabled: true,
            dir: Some(dir),
            stats: Mutex::new(CacheStats::default()),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedEntry {
    size: u64,
    mtime_ns: u64,
    count: CountResult,
    complexity: ComplexityResult,
}

fn fxhash64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0x9E3779B97F4A7C15;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::{ComplexityResult, HenryKafuraMetrics, McCabeMetrics};
    use crate::counter::CountResult;

    fn sample() -> (CountResult, ComplexityResult) {
        (
            CountResult {
                sloc: 1,
                comments: 2,
                blanks: 3,
                nodes: Default::default(),
            },
            ComplexityResult {
                halstead: Default::default(),
                mccabe: McCabeMetrics::default(),
                henry_kafura: HenryKafuraMetrics::default(),
            },
        )
    }

    #[test]
    fn put_then_get_hits() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf());
        let f = dir.path().join("a.rs");
        std::fs::write(&f, b"fn a() {}").unwrap();
        let (count, cx) = sample();
        cache.put(&f, 8, 42, &count, &cx);
        let got = cache.get(&f, 8, 42).expect("hit after put");
        assert_eq!(got.0.sloc, 1);
        assert_eq!(cache.stats(), (1, 0));
    }

    #[test]
    fn get_misses_when_mtime_differs() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf());
        let f = dir.path().join("a.rs");
        std::fs::write(&f, b"x").unwrap();
        let (count, cx) = sample();
        cache.put(&f, 1, 42, &count, &cx);
        assert!(cache.get(&f, 1, 99).is_none(), "stale mtime must miss");
        assert_eq!(cache.stats(), (0, 1));
    }

    #[test]
    fn get_missing_file_misses() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf());
        assert!(cache.get(&dir.path().join("nope.rs"), 1, 1).is_none());
        assert_eq!(cache.stats(), (0, 1));
    }

    #[test]
    fn corrupt_cache_entry_misses() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf());
        let f = dir.path().join("a.rs");
        std::fs::write(&f, b"x").unwrap();
        let ep = cache.entry_path(&f).unwrap();
        std::fs::write(ep, b"{ not json").unwrap();
        assert!(cache.get(&f, 1, 1).is_none(), "corrupt entry must miss");
    }

    #[test]
    fn disabled_cache_never_gets_or_puts() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(false);
        let f = dir.path().join("a.rs");
        std::fs::write(&f, b"x").unwrap();
        let (count, cx) = sample();
        cache.put(&f, 1, 1, &count, &cx);
        assert!(cache.get(&f, 1, 1).is_none());
        assert_eq!(cache.stats(), (0, 0));
    }

    #[test]
    fn entry_path_is_stable_and_under_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(dir.path().to_path_buf());
        let f = dir.path().join("b.rs");
        std::fs::write(&f, b"y").unwrap();
        let p1 = cache.entry_path(&f).unwrap();
        let p2 = cache.entry_path(&f).unwrap();
        assert_eq!(p1, p2);
        assert!(
            p1.starts_with(dir.path()),
            "entry must live under the cache dir"
        );
    }

    #[test]
    fn entry_path_none_when_disabled() {
        let cache = Cache::new(false);
        assert!(cache.entry_path(std::path::Path::new("x.rs")).is_none());
    }

    #[test]
    fn fxhash_deterministic() {
        assert_eq!(fxhash64(b"abc"), fxhash64(b"abc"));
        assert_ne!(fxhash64(b"abc"), fxhash64(b"abd"));
    }

    #[test]
    fn new_with_xdg_env_resolves_dir() {
        let _g = crate::TestEnvGuard::new(&["XDG_CACHE_HOME"]);
        _g.set("XDG_CACHE_HOME", "/xdg/cache");
        let cache = Cache::new(true);
        let f = std::path::Path::new("/some/project/a.rs");
        let ep = cache.entry_path(f).unwrap();
        assert!(ep.starts_with("/xdg/cache/kloc"), "{ep:?}");
        assert_eq!(cache.stats(), (0, 0));
    }

    #[test]
    fn new_disabled_has_no_dir() {
        let cache = Cache::new(false);
        assert!(cache.entry_path(std::path::Path::new("a.rs")).is_none());
    }
}
