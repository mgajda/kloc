use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

pub mod ai_config;
pub mod cache;
pub mod cli;
pub mod color;
pub mod complexity;
pub mod counter;
pub mod history;
pub mod language;
pub mod log;
pub mod output;
pub mod report;
pub mod schedule;
pub mod walker;

#[cfg(feature = "tokens")]
pub mod tokens;

use language::LanguageSpec;
pub use language::{LanguageCategory, LanguageSubgroup};
pub use report::Report;

fn normalize_name(s: &str) -> String {
    s.replace('-', "_").to_lowercase()
}

/// Serializes tests that read or write process environment variables
/// (std::env) so they cannot race when run in parallel.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that brackets a test's environment-variable changes.
///
/// Holds the process-wide `TEST_ENV_LOCK` for the whole scope and restores
/// every listed variable to its value from construction time on drop, so a
/// panicking test cannot leak environment changes into the next test.
#[cfg(test)]
#[must_use]
pub(crate) struct TestEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

#[cfg(test)]
impl TestEnvGuard {
    /// Snapshot the given variables and hold the env lock for the scope.
    pub(crate) fn new(names: &[&'static str]) -> Self {
        let _lock = TEST_ENV_LOCK.lock().unwrap();
        let saved = names
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        TestEnvGuard { _lock, saved }
    }

    pub(crate) fn set(&self, name: &'static str, value: &str) {
        unsafe { std::env::set_var(name, value) };
    }

    pub(crate) fn remove(&self, name: &'static str) {
        unsafe { std::env::remove_var(name) };
    }
}

#[cfg(test)]
impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        for (name, previous) in &self.saved {
            match previous {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

pub struct LanguageFilter {
    pub only: Vec<String>,
    pub exclude: Vec<String>,
    pub only_programming: bool,
    pub only_machine: bool,
}

impl LanguageFilter {
    pub fn matches(&self, spec: &LanguageSpec) -> bool {
        if self.only_programming && spec.category != LanguageCategory::Programming {
            return false;
        }
        if self.only_machine && spec.category != LanguageCategory::Machine {
            return false;
        }
        if !self.only.is_empty() {
            let only_normalized: Vec<String> =
                self.only.iter().map(|s| normalize_name(s)).collect();
            let ok = only_normalized.iter().any(|o| {
                o == &normalize_name(spec.name)
                    || o == spec.category_name()
                    || spec.subgroup_name().is_some_and(|s| o == s)
            });
            if !ok {
                return false;
            }
        }
        if !self.exclude.is_empty() {
            let exclude_normalized: Vec<String> =
                self.exclude.iter().map(|s| normalize_name(s)).collect();
            if exclude_normalized.iter().any(|e| {
                e == &normalize_name(spec.name)
                    || e == spec.category_name()
                    || spec.subgroup_name().is_some_and(|s| e == s)
            }) {
                return false;
            }
        }
        true
    }
}

impl From<&cli::Args> for LanguageFilter {
    fn from(args: &cli::Args) -> Self {
        LanguageFilter {
            only: args.only.clone(),
            exclude: args.exclude.clone(),
            only_programming: args.only_programming,
            only_machine: args.only_machine,
        }
    }
}

pub struct RunOptions {
    pub sloc_only: bool,
    /// Count LLM tokens (the `tokens` feature). The tokenizer takes ~2.5 s
    /// to build in debug builds; tests disable this to stay fast.
    pub count_llm_tokens: bool,
    pub cache: cache::Cache,
    pub ignore: walker::DirIgnore,
}

impl RunOptions {
    pub fn from_args(args: &cli::Args) -> Self {
        let mut ignore = walker::DirIgnore::new(!args.no_ignore_defaults);
        for name in &args.ignore {
            ignore.add(name);
        }
        for name in &args.no_ignore {
            ignore.remove(name);
        }
        RunOptions {
            sloc_only: args.sloc_only,
            // `--sloc-only` reports only SLOC/comments/blanks; it never shows
            // token counts, so skip the ~0.6 s / ~90 MB tokenizer build that
            // the default mode needs. This keeps plain counting fast.
            count_llm_tokens: !args.sloc_only,
            cache: cache::Cache::new(!args.no_cache),
            ignore,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Performance {
    pub elapsed_secs: f64,
    pub bytes_parsed: u64,
    pub files: u64,
    pub functions: u64,
    pub bytes_per_sec: f64,
    pub files_per_sec: f64,
    pub functions_per_sec: f64,
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, serde::Serialize)]
pub struct TokenCounts {
    pub deepseek_v4: u64,
    pub claude_sonnet: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FileResult {
    name: String,
    bytes: u64,
    mtime_ns: u64,
    count: counter::CountResult,
    cx: Option<complexity::ComplexityResult>,
    llm_tokens: TokenCounts,
}

pub fn run(paths: &[PathBuf], filter: &LanguageFilter, opts: &RunOptions) -> Report {
    let start = Instant::now();
    let registry = language::registry();
    let entries = walker::walk_files(paths, registry, &opts.ignore);
    let want_complexity = !opts.sloc_only;

    let results: Vec<FileResult> = entries
        .par_iter()
        .with_min_len(1)
        .filter(|entry| filter.matches(entry.language))
        .filter_map(|entry| {
            // Debug-only per-file timing; zero overhead unless debug logging
            // is enabled (-vv).
            let dbg_start = if crate::log::level() == crate::log::LogLevel::Debug {
                Some(Instant::now())
            } else {
                None
            };
            let source = std::fs::read(&entry.path).ok()?;
            let size = source.len() as u64;
            let mtime_ns = std::fs::metadata(&entry.path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);

            let count = counter::count(&source, entry.language);

            let cx = if want_complexity {
                if let Some((_, cx)) = opts.cache.get(&entry.path, size, mtime_ns) {
                    Some(cx)
                } else {
                    let cx = complexity::analyze(&source, entry.language);
                    opts.cache.put(&entry.path, size, mtime_ns, &count, &cx);
                    Some(cx)
                }
            } else {
                None
            };

            let llm_tokens = {
                #[cfg(feature = "tokens")]
                if opts.count_llm_tokens {
                    tokens::count_tokens(&source)
                } else {
                    TokenCounts::default()
                }
                #[cfg(not(feature = "tokens"))]
                {
                    TokenCounts::default()
                }
            };

            if let Some(t0) = dbg_start {
                crate::debug_log!(
                    "{:.3} s  {:>10} B  {}",
                    t0.elapsed().as_secs_f64(),
                    size,
                    entry.path.display()
                );
            }

            Some(FileResult {
                name: entry.language.name.to_string(),
                bytes: size,
                mtime_ns,
                count,
                cx,
                llm_tokens,
            })
        })
        .collect();

    let elapsed = start.elapsed().as_secs_f64();

    // Per-language aggregate: (sloc, files, comments, docs, leaf_tokens, ai_tokens).
    let mut counts: BTreeMap<String, (u64, u64, u64, u64, u64, u64)> = BTreeMap::new();
    let mut halstead_agg: BTreeMap<String, complexity::HalsteadMetrics> = BTreeMap::new();
    let mut mccabe_agg: BTreeMap<String, complexity::McCabeMetrics> = BTreeMap::new();
    let mut hk_agg = complexity::HenryKafuraMetrics::default();
    let mut nodes_agg = complexity::NodeCounts::default();
    let mut total_bytes: u64 = 0;
    let mut total_functions: u64 = 0;
    #[cfg(feature = "tokens")]
    let mut token_count = TokenCounts::default();

    for r in &results {
        total_bytes += r.bytes;
        #[cfg(feature = "tokens")]
        {
            token_count.deepseek_v4 += r.llm_tokens.deepseek_v4;
            token_count.claude_sonnet += r.llm_tokens.claude_sonnet;
        }
        nodes_agg.named_nodes += r.count.nodes.named_nodes;
        nodes_agg.leaf_tokens += r.count.nodes.leaf_tokens;
        let e = counts.entry(r.name.clone()).or_insert((0, 0, 0, 0, 0, 0));
        e.0 += r.count.sloc;
        e.1 += 1;
        e.2 += r.count.comments;
        e.3 += r.count.docs;
        e.4 += r.count.nodes.leaf_tokens;
        #[cfg(feature = "tokens")]
        {
            e.5 += r.llm_tokens.claude_sonnet;
        }

        if let Some(ref cx) = r.cx {
            let h = halstead_agg.entry(r.name.clone()).or_default();
            h.distinct_operators += cx.halstead.distinct_operators;
            h.distinct_operands += cx.halstead.distinct_operands;
            h.total_operators += cx.halstead.total_operators;
            h.total_operands += cx.halstead.total_operands;

            let m = mccabe_agg.entry(r.name.clone()).or_default();
            m.function_count += cx.mccabe.function_count;
            m.total_cyclomatic += cx.mccabe.total_cyclomatic;
            total_functions += cx.mccabe.function_count;

            hk_agg.total_modules += cx.henry_kafura.total_modules;
            hk_agg.total_fan_in += cx.henry_kafura.total_fan_in;
            hk_agg.total_fan_out += cx.henry_kafura.total_fan_out;
            hk_agg.total_information_flow += cx.henry_kafura.total_information_flow;
        }
    }

    for h in halstead_agg.values_mut() {
        h.derive();
    }

    let total_files: u64 = counts.values().map(|c| c.1).sum();

    let perf = Performance {
        elapsed_secs: elapsed,
        bytes_parsed: total_bytes,
        files: total_files,
        functions: total_functions,
        bytes_per_sec: if elapsed > 0.0 {
            total_bytes as f64 / elapsed
        } else {
            0.0
        },
        files_per_sec: if elapsed > 0.0 {
            total_files as f64 / elapsed
        } else {
            0.0
        },
        functions_per_sec: if elapsed > 0.0 {
            total_functions as f64 / elapsed
        } else {
            0.0
        },
    };

    let (cache_hits, cache_misses) = opts.cache.stats();

    crate::info_log!(
        "analyzed {} files, {} bytes in {:.3} s (cache: {} hits, {} misses)",
        total_files,
        total_bytes,
        elapsed,
        cache_hits,
        cache_misses
    );

    #[cfg(feature = "tokens")]
    let llm_tokens = Some(token_count);
    #[cfg(not(feature = "tokens"))]
    let llm_tokens = None;
    let henry_kafura = (hk_agg.total_modules > 0).then_some(hk_agg);
    Report::from_data(
        counts,
        halstead_agg,
        mccabe_agg,
        nodes_agg,
        henry_kafura,
        perf,
        llm_tokens,
        cache_hits,
        cache_misses,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LanguageSpec;
    use tree_sitter::Language;

    fn spec(
        name: &'static str,
        cat: LanguageCategory,
        sub: Option<LanguageSubgroup>,
    ) -> LanguageSpec {
        LanguageSpec {
            name,
            category: cat,
            subgroup: sub,
            extensions: &[],
            shebangs: &[],
            filenames: &[],
            grammar_fn: Some(|| Language::new(tree_sitter_rust::LANGUAGE)),
            comment_kinds: &["comment"],
        }
    }

    fn filter(only: Vec<&str>, exclude: Vec<&str>) -> LanguageFilter {
        LanguageFilter {
            only: only.into_iter().map(String::from).collect(),
            exclude: exclude.into_iter().map(String::from).collect(),
            only_programming: false,
            only_machine: false,
        }
    }

    #[test]
    fn filter_matches_programming_only() {
        let f = LanguageFilter {
            only: vec![],
            exclude: vec![],
            only_programming: true,
            only_machine: false,
        };
        let prog = spec("Rust", LanguageCategory::Programming, None);
        let machine = spec("JSON", LanguageCategory::Machine, None);
        assert!(f.matches(&prog));
        assert!(!f.matches(&machine));
    }

    #[test]
    fn filter_matches_machine_only() {
        let f = LanguageFilter {
            only: vec![],
            exclude: vec![],
            only_programming: false,
            only_machine: true,
        };
        assert!(!f.matches(&spec("Rust", LanguageCategory::Programming, None)));
        assert!(f.matches(&spec("JSON", LanguageCategory::Machine, None)));
    }

    #[test]
    fn filter_only_by_category_name_with_dash_normalizes() {
        // Category names use underscores; a user may type a dash.
        let f = filter(vec!["programming-languages"], vec![]);
        assert!(f.matches(&spec("Rust", LanguageCategory::Programming, None)));
        assert!(!f.matches(&spec("JSON", LanguageCategory::Machine, None)));
    }

    #[test]
    fn filter_only_by_language_name() {
        let f = filter(vec!["rust"], vec![]);
        assert!(f.matches(&spec("Rust", LanguageCategory::Programming, None)));
        assert!(!f.matches(&spec("Python", LanguageCategory::Programming, None)));
    }

    #[test]
    fn filter_only_by_category_and_subgroup() {
        let f = filter(vec!["programming_languages"], vec![]);
        assert!(f.matches(&spec("Rust", LanguageCategory::Programming, None)));
        let g = filter(vec!["data_languages"], vec![]);
        assert!(g.matches(&spec(
            "JSON",
            LanguageCategory::Machine,
            Some(LanguageSubgroup::Data)
        )));
        assert!(!g.matches(&spec("Rust", LanguageCategory::Programming, None)));
    }

    #[test]
    fn filter_exclude_wins_over_only() {
        let f = LanguageFilter {
            only: vec!["rust".to_string()],
            exclude: vec!["rust".to_string()],
            only_programming: false,
            only_machine: false,
        };
        assert!(!f.matches(&spec("Rust", LanguageCategory::Programming, None)));
    }

    #[test]
    fn filter_empty_matches_all() {
        let f = filter(vec![], vec![]);
        assert!(f.matches(&spec("Rust", LanguageCategory::Programming, None)));
        assert!(f.matches(&spec("JSON", LanguageCategory::Machine, None)));
    }

    #[test]
    fn runoptions_from_args() {
        use clap::Parser;
        let args = cli::Args::try_parse_from([
            "kloc",
            "--sloc-only",
            "--no-cache",
            "--ignore",
            "build",
            "--no-ignore",
            "dist",
        ])
        .unwrap();
        let opts = RunOptions::from_args(&args);
        assert!(opts.sloc_only);
        // `--sloc-only` skips the tokenizer: it never reports token counts.
        assert!(!opts.count_llm_tokens);
        assert!(opts.ignore.is_ignored("build"));
        assert!(!opts.ignore.is_ignored("dist"));
    }
}

#[test]
fn run_with_debug_logging_reports_per_file_timing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), b"fn a() {}").unwrap();
    crate::log::set_level(crate::log::LogLevel::Debug);
    let opts = RunOptions {
        sloc_only: true,
        count_llm_tokens: false,
        cache: cache::Cache::new(false),
        ignore: walker::DirIgnore::new(false),
    };
    let filter = LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let report = run(&[dir.path().to_path_buf()], &filter, &opts);
    assert!(report.total_files >= 1);
    crate::log::set_level(crate::log::LogLevel::Warning);
}

#[test]
fn run_with_complexity_and_cache_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.rs");
    std::fs::write(&f, b"fn a() {}\n").unwrap();
    let opts = RunOptions {
        sloc_only: false,
        count_llm_tokens: false,
        cache: cache::Cache::with_dir(dir.path().join("cache")),
        ignore: walker::DirIgnore::new(false),
    };
    let filter = LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let r1 = run(&[dir.path().to_path_buf()], &filter, &opts);
    assert_eq!(r1.cache_misses, 1, "first run must miss");
    let r2 = run(&[dir.path().to_path_buf()], &filter, &opts);
    assert_eq!(r2.cache_hits, 1, "second run must hit the cache");
}
#[test]
fn run_pins_exact_aggregates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        b"// hi\nfn a() {}\nfn b() { if true {} }\n",
    )
    .unwrap();
    let opts = RunOptions {
        sloc_only: false,
        count_llm_tokens: false,
        cache: cache::Cache::new(false),
        ignore: walker::DirIgnore::new(false),
    };
    let filter = LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let r = run(&[dir.path().to_path_buf()], &filter, &opts);
    assert_eq!(r.total_sloc, 2);
    assert_eq!(r.total_comments, 1);
    assert_eq!(r.total_files, 1);
    assert_eq!(r.performance.functions, 2);
    assert_eq!(r.performance.bytes_parsed, 38);
    assert!(r.performance.bytes_per_sec > 0.0);
    assert!(r.performance.files_per_sec > 0.0);
    assert!(r.performance.functions_per_sec > 0.0);
    assert_eq!(r.by_language.len(), 1);
    let rust = &r.by_language[0];
    assert_eq!(rust.name, "Rust");
    assert_eq!(
        (rust.sloc, rust.files, rust.comments, rust.leaf_tokens),
        (2, 1, 1, 17)
    );
    let h = r.halstead.as_ref().expect("halstead computed");
    assert_eq!(
        (
            h.distinct_operators,
            h.distinct_operands,
            h.total_operators,
            h.total_operands
        ),
        (9, 1, 16, 1)
    );
    let m = r.mccabe.as_ref().expect("mccabe computed");
    assert_eq!((m.function_count, m.total_cyclomatic), (2, 3));
    assert_eq!(m.average_cyclomatic, 1.5);
    let hk = r.henry_kafura.as_ref().expect("henry-kafura computed");
    assert_eq!(
        (hk.total_modules, hk.total_fan_in, hk.total_fan_out),
        (2, 0, 0)
    );
    let n = &r.nodes;
    assert_eq!((n.named_nodes, n.leaf_tokens), (14, 17));
}

#[test]
fn run_empty_file_reports_no_metrics() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.rs"), b"").unwrap();
    let opts = RunOptions {
        sloc_only: false,
        count_llm_tokens: false,
        cache: cache::Cache::new(false),
        ignore: walker::DirIgnore::new(false),
    };
    let filter = LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let r = run(&[dir.path().to_path_buf()], &filter, &opts);
    assert_eq!(r.total_sloc, 0);
    assert!(
        r.henry_kafura.is_none(),
        "no functions -> no Henry-Kafura modules"
    );
    let m = r.mccabe.expect("mccabe computed even for empty file");
    assert_eq!(m.average_cyclomatic, 0.0);
}
