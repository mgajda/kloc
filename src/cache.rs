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
        } else { None };
        if let Some(ref d) = dir {
            let _ = std::fs::create_dir_all(d);
        }
        Cache { enabled, dir, stats: Mutex::new(CacheStats::default()) }
    }

    fn entry_path(&self, path: &Path) -> Option<PathBuf> {
        let dir = self.dir.as_ref()?;
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = canonical.to_string_lossy();
        let hash = fxhash64(key.as_bytes());
        Some(dir.join(format!("{hash:016x}.json")))
    }

    pub fn get(&self, path: &Path, size: u64, mtime_ns: u64) -> Option<(CountResult, ComplexityResult)> {
        if !self.enabled { return None; }
        let ep = match self.entry_path(path) {
            Some(p) => p,
            None => { self.bump_miss(); return None; }
        };
        let bytes = match std::fs::read(&ep) {
            Ok(b) => b,
            Err(_) => { self.bump_miss(); return None; }
        };
        let entry: CachedEntry = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => { self.bump_miss(); return None; }
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

    pub fn put(&self, path: &Path, size: u64, mtime_ns: u64,
               count: &CountResult, complexity: &ComplexityResult) {
        if !self.enabled { return; }
        let Some(ep) = self.entry_path(path) else { return };
        let entry = CachedEntry { size, mtime_ns, count: *count, complexity: complexity.clone() };
        if let Ok(bytes) = serde_json::to_vec(&entry) {
            let _ = std::fs::write(ep, bytes);
        }
    }

    pub fn stats(&self) -> (u64, u64) {
        let s = self.stats.lock().unwrap();
        (s.hits, s.misses)
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
