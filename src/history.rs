//! Git-history analysis: walk a repository's commit history and count the
//! tokens changed (added + modified + removed) per language, then estimate
//! the effort and the Claude-plan time to process those tokens.
//!
//! The history is obtained by streaming `git log -p` — no library dependency,
//! so no installation, and git is guaranteed present for any repo.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{LanguageFilter, TokenCounts};

/// The token caps of an AI platform: a monotonic list of `(tokens, duration)`
/// breakpoints. The 5-hour window allowance is the first point; larger caps
/// (daily / weekly / monthly) follow. AI processing time is gated by these
/// caps (not a linear tokens-per-hour rate): the effective load is
/// decomposed into whole cap periods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCaps {
    /// `(tokens, duration_secs)` breakpoints, strictly increasing in tokens.
    pub breaks: Vec<(u64, u64)>,
}

impl AiCaps {
    /// Build caps from a config platform entry.
    pub fn from_cfg(p: &crate::ai_config::AiPlatformCfg) -> Self {
        AiCaps { breaks: p.caps.clone() }
    }
}


/// Estimated ratio of input tokens consumed while debugging to output tokens
/// written. Producing N output tokens of code typically needs 3–5× N input
/// tokens for normal projects, 10–20× for complex reasoning.
/// `effective_tokens` uses this as `(1 + multiplier)`.
pub fn effective_tokens(tokens: u64, multiplier: f64) -> u64 {
    (tokens as f64 * (1.0 + multiplier)).round() as u64
}

/// Elapsed seconds to process `tokens` of output code on the default plan
/// (Max 20x), gated by the plan caps.
pub fn ai_time_seconds(tokens: u64) -> f64 {
    let cfg = crate::ai_config::default_config();
    let p = cfg.platforms.first().expect("default config has a platform");
    let caps = AiCaps::from_cfg(p);
    let mult = p.multiplier.unwrap_or(5.0);
    ai_time_seconds_with_caps(tokens, &caps, mult)
}

/// [`ai_time_seconds`] with explicit caps and an effort multiplier
/// (`effective = tokens × (1 + multiplier)`).
pub fn ai_time_seconds_with_caps(tokens: u64, caps: &AiCaps, multiplier: f64) -> f64 {
    let effective = effective_tokens(tokens, multiplier);
    if effective == 0 { return 0.0; }
    // Find the largest cap the load fits under; whole periods + remainder.
    let mut acc_secs = 0.0;
    let mut remaining = effective;
    let mut i = caps.breaks.len();
    // Walk from the largest break down, consuming whole periods.
    while i > 0 {
        i -= 1;
        let (cap_tokens, cap_secs) = caps.breaks[i];
        if cap_tokens == 0 || cap_secs == 0 { continue; }
        let whole = remaining / cap_tokens;
        if whole > 0 {
            acc_secs += whole as f64 * cap_secs as f64;
            remaining %= cap_tokens;
        }
    }
    // Remainder is under the smallest cap; interpolate at that cap's rate.
    if remaining > 0 {
        let (small_tokens, small_secs) = caps.breaks[0];
        if small_tokens > 0 {
            acc_secs += remaining as f64 / small_tokens as f64 * small_secs as f64;
        }
    }
    acc_secs
}

/// Human-readable AI duration from an effective token load, decomposed into
/// whole cap periods (largest first) plus the remainder as minutes/seconds.
pub fn ai_duration(tokens: u64, caps: &AiCaps, multiplier: f64) -> String {
    let mut effective = effective_tokens(tokens, multiplier);
    let mut parts: Vec<String> = Vec::new();
    let mut i = caps.breaks.len();
    while i > 0 {
        i -= 1;
        let (cap_tokens, cap_secs) = caps.breaks[i];
        if cap_tokens == 0 { continue; }
        let whole = effective / cap_tokens;
        if whole > 0 {
            let unit = label_secs(cap_secs, whole);
            parts.push(unit);
            effective %= cap_tokens;
        }
    }

    // Remaining tokens under the smallest cap: express as minutes/seconds.
    if effective > 0 {
        let (small_tokens, small_secs) = caps.breaks[0];
        if small_tokens > 0 {
            let rate = small_secs as f64 / small_tokens as f64; // secs per token
            let secs = (effective as f64 * rate).round() as u64;
            if secs >= 60 {
                parts.push(format!("{} min", secs / 60));
            } else if secs > 0 {
                parts.push(format!("{secs} s"));
            }
        }
    }

    if parts.is_empty() {
        "0 s".to_string()
    } else {
        parts.join(", ")
    }
}

/// A single cap period label, e.g. "2x 5h windows", "1 day", "2 months".
/// The `x` disambiguates a count from a unit that begins with a digit.
fn label_secs(secs: u64, count: u64) -> String {
    const MIN: u64 = 60;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;
    let (name, num, digit_start) = if secs == 5 * HOUR {
        ("5h window", count, true)
    } else if secs == HOUR {
        ("hour", count, false)
    } else if secs == DAY {
        ("day", count, false)
    } else if secs == WEEK {
        ("week", count, false)
    } else if secs == MONTH {
        ("month", count, false)
    } else if secs >= MONTH {
        let months = (secs as f64 / MONTH as f64).round() as u64;
        ("month", count * months, false)
    } else if secs >= DAY {
        let days = secs / DAY;
        ("day", count * days, false)
    } else if secs >= HOUR {
        let hours = (secs as f64 / HOUR as f64).round() as u64;
        ("hour", count * hours, false)
    } else if secs >= MIN {
        let mins = secs / MIN;
        ("minute", count * mins, false)
    } else {
        ("s", count, false)
    };
    let plural = if num == 1 { "" } else { "s" };
    if digit_start {
        format!("{count}x {name}{plural}")
    } else {
        format!("{num} {name}{plural}")
    }
}



/// Per-language history totals.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LanguageHistoryTotal {
    pub name: String,
    pub files: u64,
    pub added_lines: u64,
    pub removed_lines: u64,
    pub changed_tokens: u64,
}

/// Claude-plan estimate: how many 5-hour windows and how much elapsed time
/// it would take to process the changed tokens on a given plan.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AiEstimate {
    pub platform: String,
    pub changed_tokens: u64,
    pub windows_5h: u64,
    pub elapsed_seconds: f64,
}

/// The full history report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryReport {
    pub range: String,
    pub commits: u64,
    /// Added lines in parsed languages (contribute to effort).
    pub total_added_lines: u64,
    /// Removed lines in parsed languages.
    pub total_removed_lines: u64,
    /// All added diff lines including unparsed/generated files.
    pub all_added_lines: u64,
    /// All removed diff lines including unparsed/generated files.
    pub all_removed_lines: u64,
    pub total_changed_tokens: u64,
    pub by_language: Vec<LanguageHistoryTotal>,
    pub ai_estimates: Vec<AiEstimate>,
    pub llm_changed_tokens: Option<TokenCounts>,
    /// Effort/schedule models estimated from the parsed diff-added lines.
    pub schedule: crate::schedule::ScheduleReport,
    /// Aggregated Halstead metrics from the diff added/removed source.
    pub halstead: Option<crate::complexity::HalsteadMetrics>,
    /// Human-oriented tree-sitter token count estimated from parsed diff LOC
    /// (≈4 tokens per added line — the midpoint of 200–2000 tokens per
    /// 50–500 LOC).
    pub leaf_tokens: u64,
}

/// Run the history analysis. `paths` must point inside a git work tree.
///
/// `from`/`to` select a commit range (`from..to`, or `from..` to the current
/// branch tip). With neither given, the whole history from the initial
/// commit(s) is analysed.
pub fn run_history(
    paths: &[std::path::PathBuf],
    filter: &LanguageFilter,
    from: Option<&str>,
    to: Option<&str>,
    ai_config: &crate::ai_config::AiConfig,
    ai_multiplier_override: Option<f64>,
) -> Result<HistoryReport, String> {
    let root = git_root(paths)?;
    let registry = crate::language::registry();
    let stream = git_log_p(&root, from, to)?;
    let reader = std::io::BufReader::new(stream);

    let mut commits: u64 = 0;
    // Per-commit buffers, flushed to the tokenizer at each commit boundary to
    // keep memory bounded by a single commit's diff.
    let mut per_lang: BTreeMap<String, PerLang> = BTreeMap::new();
    // Aggregated Halstead metrics per language, summed across all commits.
    let mut halstead_agg: BTreeMap<String, crate::complexity::HalsteadMetrics> = BTreeMap::new();
    // Effort-relevant lines: only parsed languages contribute to the
    // schedule estimate. Unparsed/generated files are counted separately
    // (`all_added`/`all_removed`) so the totals reflect every diff line, but
    // they never feed the COCOMO/Putnam/Halstead effort models.
    let mut total_added = 0u64;
    let mut total_removed = 0u64;
    let mut all_added = 0u64;
    let mut all_removed = 0u64;
    let mut llm = TokenCounts::default();

    let mut current_spec = None;
    let mut current_file: Option<String> = None;
    let mut new_commit = true;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("reading git log: {e}"))?;
        if let Some(hash) = line.strip_prefix("commit ") {
            if !new_commit {
                flush(&mut per_lang, &mut halstead_agg, registry, &mut llm, &mut total_added, &mut total_removed);
            }
            let _ = hash;
            commits += 1;
            new_commit = false;
        } else if let Some(p) = line.strip_prefix("diff --git ") {
            // "a/old b/new" — take the second path (b/...), the post-image.
            current_file = second_diff_path(p);
            current_spec = current_file.as_deref().and_then(|p| registry.detect_by_ext(Path::new(p)));
        } else if line.starts_with("+++") || line.starts_with("---") {
            continue;
        } else if let Some(content) = line.strip_prefix('+') {
            // Only lines within a detected, filter-matching language are
            // parsed; the rest are counted (all_added) but not in effort.
            // Blank added lines don't contribute to SLOC/effort, matching the
            // source-tree SLOC count, but their bytes still feed the token
            // and Halstead accumulators.
            let is_parsed = current_spec.is_some_and(|s| filter.matches(s));
            all_added += 1;
            if is_parsed {
                let non_blank = !content.trim().is_empty();
                let name = current_spec.unwrap().name.to_string();
                let e = per_lang.entry(name).or_default();
                if let Some(f) = &current_file {
                    e.files.insert(f.clone());
                }
                if non_blank { e.added_lines += 1; }
                e.added_bytes.push(b'\n');
                e.added_bytes.extend_from_slice(content.as_bytes());
            }
        } else if let Some(content) = line.strip_prefix('-') {
            let is_parsed = current_spec.is_some_and(|s| filter.matches(s));
            all_removed += 1;
            if is_parsed {
                let name = current_spec.unwrap().name.to_string();
                let e = per_lang.entry(name).or_default();
                if let Some(f) = &current_file {
                    e.files.insert(f.clone());
                }
                e.removed_lines += 1;
                e.removed_bytes.push(b'\n');
                e.removed_bytes.extend_from_slice(content.as_bytes());
            }
        }
    }
    flush(&mut per_lang, &mut halstead_agg, registry, &mut llm, &mut total_added, &mut total_removed);

    let total_changed_tokens = llm.claude_sonnet;
    let ai_estimates = ai_config.platforms.iter().map(|p| {
        let caps = AiCaps::from_cfg(p);
        let multiplier = ai_multiplier_override.unwrap_or(p.multiplier.unwrap_or(5.0));
        let elapsed_seconds = ai_time_seconds_with_caps(total_changed_tokens, &caps, multiplier);
        let effective = effective_tokens(total_changed_tokens, multiplier);
        let first = caps.breaks.first().map(|&(t, _)| t).unwrap_or(0);
        let windows_5h = if first > 0 { effective.div_ceil(first) } else { 0 };
        AiEstimate {
            platform: p.name.clone(),
            changed_tokens: total_changed_tokens,
            windows_5h,
            elapsed_seconds,
        }
    }).collect();

    let by_language = per_lang.into_iter().map(|(name, p)| LanguageHistoryTotal {
        name,
        files: p.files.len() as u64,
        added_lines: p.total_added_lines,
        removed_lines: p.total_removed_lines,
        changed_tokens: p.added_tokens + p.removed_tokens,
    }).collect();

    let halstead = aggregate_halstead(&halstead_agg);

    Ok(HistoryReport {
        range: build_range(from, to).unwrap_or_else(|| "full history".to_string()),
        commits,
        total_added_lines: total_added,
        total_removed_lines: total_removed,
        all_added_lines: all_added,
        all_removed_lines: all_removed,
        total_changed_tokens,
        by_language,
        ai_estimates,
        llm_changed_tokens: {
            #[cfg(feature = "tokens")]
            { Some(llm) }
            #[cfg(not(feature = "tokens"))]
            { None }
        },
        schedule: crate::schedule::estimate(total_added, halstead.as_ref().map_or(0.0, |h| h.effort)),
        halstead,
        // ~4 tree-sitter tokens per added LOC (midpoint of 200–2000 tokens
        // per 50–500 LOC/day).
        leaf_tokens: total_added * 4,
    })
}

/// Sum per-language Halstead operator/operand counts and derive the derived
/// metrics (volume, difficulty, effort, time, bugs).
fn aggregate_halstead(
    agg: &BTreeMap<String, crate::complexity::HalsteadMetrics>,
) -> Option<crate::complexity::HalsteadMetrics> {
    crate::complexity::aggregate_halstead(agg.values())
}

#[derive(Default)]
struct PerLang {
    added_lines: u64,
    removed_lines: u64,
    added_bytes: Vec<u8>,
    removed_bytes: Vec<u8>,
    added_tokens: u64,
    removed_tokens: u64,
    total_added_lines: u64,
    total_removed_lines: u64,
    files: std::collections::HashSet<String>,
}

/// Flush a commit's buffered bytes through the LLM tokenizer and fold the
/// line/token counts into the running totals. Resets each language's byte
/// buffers so the next commit starts clean (memory stays bounded by one
/// commit's diff).
fn flush(
    per_lang: &mut BTreeMap<String, PerLang>,
    halstead_agg: &mut BTreeMap<String, crate::complexity::HalsteadMetrics>,
    registry: &crate::language::LanguageRegistry,
    _llm: &mut TokenCounts,
    total_added: &mut u64,
    total_removed: &mut u64,
) {
    for (name, p) in per_lang.iter_mut() {
        *total_added += p.added_lines;
        *total_removed += p.removed_lines;
        p.total_added_lines += p.added_lines;
        p.total_removed_lines += p.removed_lines;
        // Halstead: analyse the added/removed source with tree-sitter and
        // sum operators/operands into the per-language aggregate.
        if let Some(spec) = registry.find_by_name(name) {
            let added = crate::complexity::analyze(&p.added_bytes, spec);
            let removed = crate::complexity::analyze(&p.removed_bytes, spec);
            let h = halstead_agg.entry(name.clone()).or_default();
            h.distinct_operators += added.halstead.distinct_operators + removed.halstead.distinct_operators;
            h.distinct_operands += added.halstead.distinct_operands + removed.halstead.distinct_operands;
            h.total_operators += added.halstead.total_operators + removed.halstead.total_operators;
            h.total_operands += added.halstead.total_operands + removed.halstead.total_operands;
        }
        #[cfg(feature = "tokens")]
        {
            let a = crate::tokens::count_tokens(&p.added_bytes);
            let r = crate::tokens::count_tokens(&p.removed_bytes);
            p.added_tokens += a.claude_sonnet;
            p.removed_tokens += r.claude_sonnet;
            _llm.claude_sonnet += a.claude_sonnet + r.claude_sonnet;
            _llm.deepseek_v4 += a.deepseek_v4 + r.deepseek_v4;
        }
        p.added_bytes.clear();
        p.removed_bytes.clear();
        p.added_lines = 0;
        p.removed_lines = 0;
    }
}

/// Run `git log -p` and return its stdout as a pipe.
///
/// `from`/`to` build a git revision range using native git semantics
/// (`from` is exclusive): `from..to`, `from..` (to the current branch tip),
/// or no revision argument (full history from the initial commit(s)).
fn git_log_p(root: &Path, from: Option<&str>, to: Option<&str>) -> Result<impl std::io::Read, String> {
    let mut cmd = Command::new("git");
    cmd.arg("log")
        .arg("-p")
        .arg("--format=commit %H")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(range) = build_range(from, to) {
        cmd.arg(range);
    }
    let child = cmd.spawn().map_err(|e| format!("failed to run git log: {e}"))?;
    Ok(child.stdout.ok_or("git log produced no stdout")?)
}

/// Build the git revision-range argument.
///
/// - `(None, None)` → `None` (full history).
/// - `(Some(f), None)` → `f..` (from `f` to the current branch tip).
/// - `(Some(f), Some(t))` → `f..t`.
/// - `(None, Some(t))` → `t` (everything reachable from `t`).
fn build_range(from: Option<&str>, to: Option<&str>) -> Option<String> {
    match (from, to) {
        (None, None) => None,
        (Some(f), None) => Some(format!("{f}..")),
        (Some(f), Some(t)) => Some(format!("{f}..{t}")),
        (None, Some(t)) => Some(t.to_string()),
    }
}

/// Find the git work-tree root from the given paths (fall back to cwd).
fn git_root(paths: &[std::path::PathBuf]) -> Result<std::path::PathBuf, String> {
    let start = paths.first().cloned().unwrap_or_else(|| std::path::PathBuf::from("."));
    let out = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(start)
        .output()
        .map_err(|e| format!("not a git repository ({e}); --history requires git"))?;
    if !out.status.success() {
        return Err("--history requires a git repository (git rev-parse failed)".to_string());
    }
    Ok(std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// Extract the post-image path (`b/...`) from a `diff --git a/X b/Y` line.
fn second_diff_path(p: &str) -> Option<String> {
    let mut it = p.split_whitespace();
    let _a = it.next();
    let b = it.next()?;
    b.strip_prefix("b/").map(|s| s.to_string()).or_else(|| Some(b.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_tokens_debug_multiplier() {
        // N output tokens → N × (1 + multiplier) effective (default 5x input).
        assert_eq!(effective_tokens(0, 5.0), 0);
        assert_eq!(effective_tokens(100, 5.0), 600);
        assert_eq!(effective_tokens(1000, 5.0), 6000);
        // Complex reasoning: 10x multiplier.
        assert_eq!(effective_tokens(100, 10.0), 1100);
    }

    #[test]
    fn test_ai_caps_from_config() {
        // Default config: first platform exists and has monotonic caps.
        let cfg = crate::ai_config::default_config();
        assert!(!cfg.platforms.is_empty());
        let p = &cfg.platforms[0];
        let caps = AiCaps::from_cfg(p);
        assert!(!caps.breaks.is_empty());
        // monotonic: token caps strictly increase
        for w in caps.breaks.windows(2) {
            assert!(w[0].0 < w[1].0, "tokens must increase");
            assert!(w[0].1 < w[1].1, "durations must increase");
        }
    }

    #[test]
    fn test_ai_time_seconds_by_caps() {
        // Default config first platform: effective = tokens × 6 (5x).
        // With 50k output → 300k effective; 300k / first-cap(44k) ≈ 7 windows.
        let secs = ai_time_seconds(50_000);
        assert!(secs > 0.0, "expected >0, got {secs}");
    }

    #[test]
    fn test_ai_duration_units() {
        let cfg = crate::ai_config::default_config();
        let p = &cfg.platforms[0];
        let caps = AiCaps::from_cfg(p);
        let mult = p.multiplier.unwrap_or(5.0);
        // Zero output → "0 s".
        assert_eq!(ai_duration(0, &caps, mult), "0 s");
        // Small output → a window/minutes label, non-empty.
        let d = ai_duration(10_000, &caps, mult);
        assert!(!d.is_empty());
        // The schedule table AI columns must render the platform label.
        assert!(!p.name.is_empty());
    }

    #[test]
    fn test_build_range() {
        assert_eq!(build_range(None, None), None);
        assert_eq!(build_range(Some("v1"), None), Some("v1..".to_string()));
        assert_eq!(build_range(Some("v1"), Some("v2")), Some("v1..v2".to_string()));
        assert_eq!(build_range(None, Some("HEAD")), Some("HEAD".to_string()));
    }

    #[test]
    fn test_second_diff_path() {
        assert_eq!(second_diff_path("a/old.rs b/new.rs"), Some("new.rs".to_string()));
        assert_eq!(second_diff_path("b/new.rs b/new.rs"), Some("new.rs".to_string()));
    }
}
