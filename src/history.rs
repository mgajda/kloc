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

/// Claude subscription plans used as AI time-to-process calibration points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum AiPlan {
    Pro,
    Max5,
    Max20,
}

impl AiPlan {
    /// Approximate token allowance per rolling 5-hour window.
    ///
    /// Anthropic's help centre publishes only the relative multiples (Max 5x
    /// is 5× Pro, Max 20x is 20× Pro), not absolute token numbers. The Pro
    /// baseline (~44k tokens / 5h) is the widely-reported figure (faros.ai,
    /// Dec 2025; Claude Code 5-hour limits were doubled in May 2026, so the
    /// current allowance may be higher). Treat these as rough calibration,
    /// and override with `--ai-budget`.
    pub fn tokens_per_5h(self) -> u64 {
        match self {
            AiPlan::Pro => 44_000,
            AiPlan::Max5 => 88_000,
            AiPlan::Max20 => 220_000,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AiPlan::Pro => "Claude Pro",
            AiPlan::Max5 => "Claude Max 5x",
            AiPlan::Max20 => "Claude Max 20x",
        }
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
    pub plan: &'static str,
    pub tokens_per_5h: u64,
    pub changed_tokens: u64,
    pub windows_5h: u64,
    pub elapsed_seconds: f64,
}

/// The full history report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryReport {
    pub commits: u64,
    pub total_added_lines: u64,
    pub total_removed_lines: u64,
    pub total_changed_tokens: u64,
    pub by_language: Vec<LanguageHistoryTotal>,
    pub ai_estimates: Vec<AiEstimate>,
    pub llm_changed_tokens: Option<TokenCounts>,
}

/// Run the history analysis. `paths` must point inside a git work tree.
pub fn run_history(
    paths: &[std::path::PathBuf],
    filter: &LanguageFilter,
    ai_plans: &[AiPlan],
    ai_budget_override: Option<u64>,
) -> Result<HistoryReport, String> {
    let root = git_root(paths)?;
    let registry = crate::language::registry();
    let stream = git_log_p(&root)?;
    let reader = std::io::BufReader::new(stream);

    let mut commits: u64 = 0;
    // Per-commit buffers, flushed to the tokenizer at each commit boundary to
    // keep memory bounded by a single commit's diff.
    let mut per_lang: BTreeMap<String, PerLang> = BTreeMap::new();
    let mut total_added = 0u64;
    let mut total_removed = 0u64;
    let mut llm = TokenCounts::default();

    let mut current_spec = None;
    let mut current_file: Option<String> = None;
    let mut new_commit = true;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("reading git log: {e}"))?;
        if let Some(hash) = line.strip_prefix("commit ") {
            if !new_commit {
                flush(&mut per_lang, &mut llm, &mut total_added, &mut total_removed);
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
            if let Some(spec) = current_spec
                && filter.matches(spec) {
                    let name = spec.name.to_string();
                    let e = per_lang.entry(name).or_default();
                    if let Some(f) = &current_file {
                        e.files.insert(f.clone());
                    }
                    e.added_lines += 1;
                    e.added_bytes.push(b'\n');
                    e.added_bytes.extend_from_slice(content.as_bytes());
                }
        } else if let Some(content) = line.strip_prefix('-')
            && let Some(spec) = current_spec
                && filter.matches(spec) {
                    let name = spec.name.to_string();
                    let e = per_lang.entry(name).or_default();
                    if let Some(f) = &current_file {
                        e.files.insert(f.clone());
                    }
                    e.removed_lines += 1;
                    e.removed_bytes.push(b'\n');
                    e.removed_bytes.extend_from_slice(content.as_bytes());
                }
    }
    flush(&mut per_lang, &mut llm, &mut total_added, &mut total_removed);

    let total_changed_tokens = llm.claude_sonnet;
    let ai_estimates = ai_plans.iter().map(|plan| {
        let budget = ai_budget_override.unwrap_or_else(|| plan.tokens_per_5h());
        let windows_5h = if budget > 0 { total_changed_tokens.div_ceil(budget) } else { 0 };
        let elapsed_seconds = windows_5h as f64 * 5.0 * 3600.0;
        AiEstimate { plan: plan.label(), tokens_per_5h: budget, changed_tokens: total_changed_tokens, windows_5h, elapsed_seconds }
    }).collect();

    let by_language = per_lang.into_iter().map(|(name, p)| LanguageHistoryTotal {
        name,
        files: p.files.len() as u64,
        added_lines: p.total_added_lines,
        removed_lines: p.total_removed_lines,
        changed_tokens: p.added_tokens + p.removed_tokens,
    }).collect();

    Ok(HistoryReport {
        commits,
        total_added_lines: total_added,
        total_removed_lines: total_removed,
        total_changed_tokens,
        by_language,
        ai_estimates,
        llm_changed_tokens: {
            #[cfg(feature = "tokens")]
            { Some(llm) }
            #[cfg(not(feature = "tokens"))]
            { None }
        },
    })
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
    llm: &mut TokenCounts,
    total_added: &mut u64,
    total_removed: &mut u64,
) {
    for p in per_lang.values_mut() {
        *total_added += p.added_lines;
        *total_removed += p.removed_lines;
        p.total_added_lines += p.added_lines;
        p.total_removed_lines += p.removed_lines;
        #[cfg(feature = "tokens")]
        {
            let a = crate::tokens::count_tokens(&p.added_bytes);
            let r = crate::tokens::count_tokens(&p.removed_bytes);
            p.added_tokens += a.claude_sonnet;
            p.removed_tokens += r.claude_sonnet;
            llm.claude_sonnet += a.claude_sonnet + r.claude_sonnet;
            llm.deepseek_v4 += a.deepseek_v4 + r.deepseek_v4;
        }
        p.added_bytes.clear();
        p.removed_bytes.clear();
        p.added_lines = 0;
        p.removed_lines = 0;
    }
}

/// Run `git log -p` and return its stdout as a pipe.
fn git_log_p(root: &Path) -> Result<impl std::io::Read, String> {
    let child = Command::new("git")
        .arg("log")
        .arg("-p")
        .arg("--format=commit %H")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run git log: {e}"))?;
    Ok(child.stdout.ok_or("git log produced no stdout")?)
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
