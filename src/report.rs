use std::collections::BTreeMap;
use serde::Serialize;
use crate::complexity;

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
}

impl Report {
    pub fn from_data(
        counts: BTreeMap<String, (u64, u64, u64)>,
        cx_halstead: BTreeMap<String, complexity::HalsteadMetrics>,
        cx_mccabe: BTreeMap<String, complexity::McCabeMetrics>,
        _cx_hk: BTreeMap<String, complexity::HenryKafuraMetrics>,
    ) -> Self {
        let mut by_language: Vec<LanguageTotal> = counts
            .into_iter()
            .map(|(name, (sloc, files, comments))| LanguageTotal { name, sloc, comments, files })
            .collect();
        by_language.sort_by(|a, b| b.sloc.cmp(&a.sloc));

        let total_sloc = by_language.iter().map(|l| l.sloc).sum();
        let total_comments = by_language.iter().map(|l| l.comments).sum();
        let total_files = by_language.iter().map(|l| l.files).sum();

        let total_halstead = cx_halstead.values().fold(
            complexity::HalsteadMetrics {
                distinct_operators: 0, distinct_operands: 0,
                total_operators: 0, total_operands: 0,
                vocabulary: 0, length: 0, estimated_length: 0.0,
                volume: 0.0, difficulty: 0.0, effort: 0.0,
                time_seconds: 0.0, bugs: 0.0,
            },
            |mut acc, h| {
                acc.distinct_operators += h.distinct_operators;
                acc.distinct_operands += h.distinct_operands;
                acc.total_operators += h.total_operators;
                acc.total_operands += h.total_operands;
                let n1 = acc.distinct_operators; let n2 = acc.distinct_operands;
                let t1 = acc.total_operators; let t2 = acc.total_operands;
                let n_vocab = n1 + n2; let n_len = t1 + t2;
                let volume = if n_vocab > 0 { (n_len as f64) * (n_vocab as f64).log2() } else { 0.0 };
                let diff = if n1 > 0 { (n1 as f64 / 2.0) * (t2 as f64 / n2.max(1) as f64) } else { 0.0 };
                let est_len = if n1 > 0 && n2 > 0 {
                    (n1 as f64) * (n1 as f64).log2() + (n2 as f64) * (n2 as f64).log2()
                } else { 0.0 };
                acc.vocabulary = n_vocab; acc.length = n_len;
                acc.estimated_length = est_len;
                acc.volume = volume; acc.difficulty = diff;
                acc.effort = diff * volume;
                acc.time_seconds = acc.effort / 18.0;
                acc.bugs = volume / 3000.0;
                acc
            },
        );

        let total_mccabe = cx_mccabe.values().fold(
            complexity::McCabeMetrics {
                function_count: 0, total_cyclomatic: 0, average_cyclomatic: 0.0,
            },
            |mut acc, m| {
                acc.function_count += m.function_count;
                acc.total_cyclomatic += m.total_cyclomatic;
                acc
            },
        );
        let total_mccabe = complexity::McCabeMetrics {
            average_cyclomatic: if total_mccabe.function_count > 0 {
                total_mccabe.total_cyclomatic as f64 / total_mccabe.function_count as f64
            } else { 0.0 },
            ..total_mccabe
        };

        Report {
            by_language,
            total_sloc, total_comments, total_files,
            halstead: Some(total_halstead),
            mccabe: Some(total_mccabe),
        }
    }
}
