use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;
use rayon::prelude::*;

pub mod language;
pub mod counter;
pub mod walker;
pub mod output;
pub mod report;
pub mod cli;
pub mod complexity;
pub mod schedule;
pub mod cache;
pub mod color;
pub mod history;
pub mod ai_config;
pub mod log;

#[cfg(feature = "tokens")]
pub mod tokens;

pub use report::Report;
pub use language::{LanguageCategory, LanguageSubgroup};
use language::LanguageSpec;

fn normalize_name(s: &str) -> String {
    s.replace('-', "_").to_lowercase()
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
            let only_normalized: Vec<String> = self.only.iter().map(|s| normalize_name(s)).collect();
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
            let exclude_normalized: Vec<String> = self.exclude.iter().map(|s| normalize_name(s)).collect();
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

#[derive(Debug, Clone, Default, Copy, serde::Serialize)]
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
            let mtime_ns = std::fs::metadata(&entry.path).ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64).unwrap_or(0);

            let count = counter::count(&source, entry.language);

            let cx = if want_complexity {
                if let Some((_, cx)) = opts.cache.get(&entry.path, size, mtime_ns) {
                    Some(cx)
                } else {
                    let cx = complexity::analyze(&source, entry.language);
                    opts.cache.put(&entry.path, size, mtime_ns, &count, &cx);
                    Some(cx)
                }
            } else { None };

            let llm_tokens = {
                #[cfg(feature = "tokens")]
                { tokens::count_tokens(&source) }
                #[cfg(not(feature = "tokens"))]
                { TokenCounts::default() }
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
                count, cx, llm_tokens,
            })
        })
        .collect();

    let elapsed = start.elapsed().as_secs_f64();

    // Per-language aggregate: (sloc, files, comments, leaf_tokens, ai_tokens).
    let mut counts: BTreeMap<String, (u64, u64, u64, u64, u64)> = BTreeMap::new();
    let mut halstead_agg: BTreeMap<String, complexity::HalsteadMetrics> = BTreeMap::new();
    let mut mccabe_agg: BTreeMap<String, complexity::McCabeMetrics> = BTreeMap::new();
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
        let e = counts.entry(r.name.clone()).or_insert((0, 0, 0, 0, 0));
        e.0 += r.count.sloc; e.1 += 1; e.2 += r.count.comments;
        e.3 += r.count.nodes.leaf_tokens;
        #[cfg(feature = "tokens")]
        { e.4 += r.llm_tokens.claude_sonnet; }

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
        bytes_per_sec: if elapsed > 0.0 { total_bytes as f64 / elapsed } else { 0.0 },
        files_per_sec: if elapsed > 0.0 { total_files as f64 / elapsed } else { 0.0 },
        functions_per_sec: if elapsed > 0.0 { total_functions as f64 / elapsed } else { 0.0 },
    };

    let (cache_hits, cache_misses) = opts.cache.stats();

    crate::info_log!(
        "analyzed {} files, {} bytes in {:.3} s (cache: {} hits, {} misses)",
        total_files, total_bytes, elapsed, cache_hits, cache_misses
    );

    #[cfg(feature = "tokens")]
    let llm_tokens = Some(token_count);
    #[cfg(not(feature = "tokens"))]
    let llm_tokens = None;
    Report::from_data(
        counts, halstead_agg, mccabe_agg, nodes_agg,
        perf, llm_tokens, cache_hits, cache_misses,
    )
}

