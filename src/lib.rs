use std::collections::BTreeMap;
use std::path::PathBuf;
use rayon::prelude::*;

pub mod language;
pub mod counter;
pub mod walker;
pub mod output;
pub mod report;
pub mod cli;
pub mod complexity;

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

pub fn run(paths: &[PathBuf], filter: &LanguageFilter) -> Report {
    let registry = language::registry();

    let entries = walker::walk_files(paths, registry);

    let results: Vec<(String, counter::CountResult, complexity::ComplexityResult)> = entries
        .par_iter()
        .with_min_len(1)
        .filter(|entry| filter.matches(entry.language))
        .filter_map(|entry| {
            let source = std::fs::read(&entry.path).ok()?;
            let count = counter::count(&source, entry.language);
            let cx = complexity::analyze(&source, entry.language);
            Some((entry.language.name.to_string(), count, cx))
        })
        .collect();

    let mut counts: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    let mut cx_halstead: BTreeMap<String, complexity::HalsteadMetrics> = BTreeMap::new();
    let mut cx_mccabe: BTreeMap<String, complexity::McCabeMetrics> = BTreeMap::new();
    let mut cx_hk: BTreeMap<String, complexity::HenryKafuraMetrics> = BTreeMap::new();

    for (name, count, cx) in results {
        let e = counts.entry(name.clone()).or_insert((0, 0, 0));
        e.0 += count.sloc; e.1 += 1; e.2 += count.comments;

        let h = cx_halstead.entry(name.clone()).or_default();
        h.distinct_operators += cx.halstead.distinct_operators;
        h.distinct_operands += cx.halstead.distinct_operands;
        h.total_operators += cx.halstead.total_operators;
        h.total_operands += cx.halstead.total_operands;

        let m = cx_mccabe.entry(name.clone()).or_insert_with(|| complexity::McCabeMetrics {
            function_count: 0, total_cyclomatic: 0, average_cyclomatic: 0.0,
        });
        m.function_count += cx.mccabe.function_count;
        m.total_cyclomatic += cx.mccabe.total_cyclomatic;

        let k = cx_hk.entry(name.clone()).or_insert_with(|| cx.henry_kafura.clone());
        k.total_modules += cx.henry_kafura.total_modules;
        k.total_fan_in += cx.henry_kafura.total_fan_in;
        k.total_fan_out += cx.henry_kafura.total_fan_out;
    }

    for (name, h) in &mut cx_halstead {
        let n1 = h.distinct_operators; let n2 = h.distinct_operands;
        let t1 = h.total_operators; let t2 = h.total_operands;
        let n_vocab = n1 + n2; let n_len = t1 + t2;
        let volume = if n_vocab > 0 { (n_len as f64) * (n_vocab as f64).log2() } else { 0.0 };
        let diff = if n1 > 0 { (n1 as f64 / 2.0) * (t2 as f64 / n2.max(1) as f64) } else { 0.0 };
        h.vocabulary = n_vocab; h.length = n_len;
        h.estimated_length = if n1 > 0 && n2 > 0 {
            n1 as f64 * (n1 as f64).log2() + n2 as f64 * (n2 as f64).log2()
        } else { 0.0 };
        h.volume = volume; h.difficulty = diff;
        h.effort = diff * volume;
        h.time_seconds = h.effort / 18.0;
        h.bugs = volume / 3000.0;
    }

    for (_, m) in &mut cx_mccabe {
        m.average_cyclomatic = if m.function_count > 0 {
            m.total_cyclomatic as f64 / m.function_count as f64
        } else { 0.0 };
    }

    Report::from_data(counts, cx_halstead, cx_mccabe, cx_hk)
}
