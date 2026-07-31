use std::collections::BTreeMap;
use serde::Serialize;
use crate::complexity;
use crate::schedule;
use crate::Performance;

#[derive(Debug, Clone, Serialize)]
pub struct LanguageTotal {
    pub name: String,
    pub sloc: u64,
    pub comments: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub by_language: Vec<LanguageTotal>,
    pub total_sloc: u64,
    pub total_comments: u64,
    pub total_files: u64,
    pub halstead: Option<complexity::HalsteadMetrics>,
    pub mccabe: Option<complexity::McCabeMetrics>,
    pub nodes: Option<complexity::NodeCounts>,
    pub schedule: Option<schedule::ScheduleReport>,
    pub tokens: Option<u64>,
    pub performance: Performance,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Report {
    #[allow(clippy::too_many_arguments)]
    pub fn from_data(
        counts: BTreeMap<String, (u64, u64, u64)>,
        cx_halstead: BTreeMap<String, complexity::HalsteadMetrics>,
        cx_mccabe: BTreeMap<String, complexity::McCabeMetrics>,
        nodes: complexity::NodeCounts,
        performance: Performance,
        tokens: u64,
        cache_hits: u64,
        cache_misses: u64,
    ) -> Self {
        let mut by_language: Vec<LanguageTotal> = counts
            .into_iter()
            .map(|(name, (sloc, files, comments))| LanguageTotal { name, sloc, comments, files })
            .collect();
        by_language.sort_by(|a, b| b.sloc.cmp(&a.sloc));

        let total_sloc = by_language.iter().map(|l| l.sloc).sum();
        let total_comments = by_language.iter().map(|l| l.comments).sum();
        let total_files = by_language.iter().map(|l| l.files).sum();

        let halstead = aggregate_halstead(&cx_halstead);
        let mccabe = aggregate_mccabe(&cx_mccabe);

        let schedule = halstead.as_ref().map(|h| {
            schedule::estimate(total_sloc, h.effort)
        });

        Report {
            by_language,
            total_sloc, total_comments, total_files,
            halstead,
            mccabe,
            nodes: if nodes.named_nodes > 0 || nodes.leaf_tokens > 0 { Some(nodes) } else { None },
            schedule,
            tokens: if tokens > 0 { Some(tokens) } else { None },
            performance,
            cache_hits, cache_misses,
        }
    }
}

fn aggregate_halstead(
    cx_halstead: &BTreeMap<String, complexity::HalsteadMetrics>,
) -> Option<complexity::HalsteadMetrics> {
    if cx_halstead.is_empty() { return None; }
    let mut acc = complexity::HalsteadMetrics::default();
    for h in cx_halstead.values() {
        acc.distinct_operators += h.distinct_operators;
        acc.distinct_operands += h.distinct_operands;
        acc.total_operators += h.total_operators;
        acc.total_operands += h.total_operands;
    }
    let n1 = acc.distinct_operators; let n2 = acc.distinct_operands;
    let t1 = acc.total_operators; let t2 = acc.total_operands;
    let n_vocab = n1 + n2; let n_len = t1 + t2;
    let volume = if n_vocab > 0 { (n_len as f64) * (n_vocab as f64).log2() } else { 0.0 };
    let diff = if n1 > 0 { (n1 as f64 / 2.0) * (t2 as f64 / n2.max(1) as f64) } else { 0.0 };
    acc.vocabulary = n_vocab; acc.length = n_len;
    acc.estimated_length = if n1 > 0 && n2 > 0 {
        n1 as f64 * (n1 as f64).log2() + n2 as f64 * (n2 as f64).log2()
    } else { 0.0 };
    acc.volume = volume; acc.difficulty = diff;
    acc.effort = diff * volume;
    acc.time_seconds = acc.effort / 18.0;
    acc.bugs = volume / 3000.0;
    Some(acc)
}

fn aggregate_mccabe(
    cx_mccabe: &BTreeMap<String, complexity::McCabeMetrics>,
) -> Option<complexity::McCabeMetrics> {
    if cx_mccabe.is_empty() { return None; }
    let mut acc = complexity::McCabeMetrics { function_count: 0, total_cyclomatic: 0, average_cyclomatic: 0.0 };
    for m in cx_mccabe.values() {
        acc.function_count += m.function_count;
        acc.total_cyclomatic += m.total_cyclomatic;
    }
    acc.average_cyclomatic = if acc.function_count > 0 {
        acc.total_cyclomatic as f64 / acc.function_count as f64
    } else { 0.0 };
    Some(acc)
}
