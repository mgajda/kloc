use std::collections::BTreeMap;
use serde::Serialize;

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
}

impl Report {
    pub fn from_counts(counts: BTreeMap<String, (u64, u64, u64)>) -> Self {
        let mut by_language: Vec<LanguageTotal> = counts
            .into_iter()
            .map(|(name, (sloc, files, comments))| LanguageTotal { name, sloc, comments, files })
            .collect();
        by_language.sort_by(|a, b| b.sloc.cmp(&a.sloc));

        let total_sloc = by_language.iter().map(|l| l.sloc).sum();
        let total_comments = by_language.iter().map(|l| l.comments).sum();
        let total_files = by_language.iter().map(|l| l.files).sum();

        Report { by_language, total_sloc, total_comments, total_files }
    }
}
