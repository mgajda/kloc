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

pub fn run(paths: &[PathBuf]) -> Report {
    let registry = language::registry();

    let entries = walker::walk_files(paths, registry);

    let counts: Vec<(String, counter::CountResult)> = entries
        .par_iter()
        .with_min_len(1)
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
