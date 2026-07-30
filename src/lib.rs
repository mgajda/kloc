use std::collections::BTreeMap;
use std::path::PathBuf;
use rayon::prelude::*;

pub mod language;
pub mod counter;
pub mod walker;
pub mod output;
pub mod report;
pub mod cli;

pub use report::Report;
pub use language::LanguageCategory;
use language::LanguageSpec;

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
        if !self.only.is_empty() && !self.only.iter().any(|o| spec.name.eq_ignore_ascii_case(o)) {
            return false;
        }
        if !self.exclude.is_empty() && self.exclude.iter().any(|e| spec.name.eq_ignore_ascii_case(e)) {
            return false;
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

    let counts: Vec<(String, counter::CountResult)> = entries
        .par_iter()
        .with_min_len(1)
        .filter(|entry| filter.matches(entry.language))
        .filter_map(|entry| {
            let source = std::fs::read(&entry.path).ok()?;
            let result = counter::count(&source, entry.language);
            Some((entry.language.name.to_string(), result))
        })
        .collect();

    let mut aggregated: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for (name, result) in counts {
        let entry = aggregated.entry(name).or_insert((0, 0));
        entry.0 += result.sloc;
        entry.1 += 1;
    }

    Report::from_counts(aggregated)
}
