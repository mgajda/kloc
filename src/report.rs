use crate::complexity;
use crate::schedule;
use crate::{Performance, TokenCounts};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct LanguageTotal {
    pub name: String,
    pub sloc: u64,
    pub comments: u64,
    /// Documentation lines (docstrings) — counted separately, never in `sloc`.
    pub docs: u64,
    pub files: u64,
    pub leaf_tokens: u64,
    pub ai_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub by_language: Vec<LanguageTotal>,
    pub total_sloc: u64,
    pub total_comments: u64,
    pub total_docs: u64,
    pub total_files: u64,
    pub halstead: Option<complexity::HalsteadMetrics>,
    pub mccabe: Option<complexity::McCabeMetrics>,
    pub henry_kafura: Option<complexity::HenryKafuraMetrics>,
    pub nodes: complexity::NodeCounts,
    pub schedule: Option<schedule::ScheduleReport>,
    pub llm_tokens: Option<TokenCounts>,
    pub performance: Performance,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Report {
    #[allow(clippy::too_many_arguments)]
    pub fn from_data(
        counts: BTreeMap<String, (u64, u64, u64, u64, u64, u64)>,
        cx_halstead: BTreeMap<String, complexity::HalsteadMetrics>,
        cx_mccabe: BTreeMap<String, complexity::McCabeMetrics>,
        nodes: complexity::NodeCounts,
        henry_kafura: Option<complexity::HenryKafuraMetrics>,
        performance: Performance,
        llm_tokens: Option<TokenCounts>,
        cache_hits: u64,
        cache_misses: u64,
    ) -> Self {
        let mut by_language: Vec<LanguageTotal> = counts
            .into_iter()
            .map(
                |(name, (sloc, files, comments, docs, leaf, ai))| LanguageTotal {
                    name,
                    sloc,
                    comments,
                    docs,
                    files,
                    leaf_tokens: leaf,
                    ai_tokens: ai,
                },
            )
            .collect();
        by_language.sort_by_key(|l| std::cmp::Reverse(l.sloc));

        let total_sloc = by_language.iter().map(|l| l.sloc).sum();
        let total_comments = by_language.iter().map(|l| l.comments).sum();
        let total_docs = by_language.iter().map(|l| l.docs).sum();
        let total_files = by_language.iter().map(|l| l.files).sum();

        let halstead = complexity::aggregate_halstead(cx_halstead.values());
        let mccabe = aggregate_mccabe(&cx_mccabe);

        let schedule = halstead
            .as_ref()
            .map(|h| schedule::estimate(total_sloc, h.effort));

        Report {
            by_language,
            total_sloc,
            total_comments,
            total_docs,
            total_files,
            halstead,
            mccabe,
            henry_kafura,
            nodes,
            schedule,
            llm_tokens,
            performance,
            cache_hits,
            cache_misses,
        }
    }
}

fn aggregate_mccabe(
    cx_mccabe: &BTreeMap<String, complexity::McCabeMetrics>,
) -> Option<complexity::McCabeMetrics> {
    if cx_mccabe.is_empty() {
        return None;
    }
    let mut acc = complexity::McCabeMetrics {
        function_count: 0,
        total_cyclomatic: 0,
        average_cyclomatic: 0.0,
    };
    for m in cx_mccabe.values() {
        acc.function_count += m.function_count;
        acc.total_cyclomatic += m.total_cyclomatic;
    }
    acc.average_cyclomatic = if acc.function_count > 0 {
        acc.total_cyclomatic as f64 / acc.function_count as f64
    } else {
        0.0
    };
    Some(acc)
}
