use std::collections::HashSet;
use tree_sitter::{Node, Parser};
use crate::language::LanguageSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountResult {
    pub sloc: u64,
    pub comments: u64,
    pub blanks: u64,
}

impl CountResult {
    pub fn total(&self) -> u64 {
        self.sloc + self.comments + self.blanks
    }
}

pub fn count(source: &[u8], spec: &LanguageSpec) -> CountResult {
    let mut parser = Parser::new();
    if parser.set_language(&spec.grammar()).is_err() {
        return CountResult { sloc: 0, comments: 0, blanks: line_count(source) as u64 };
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return CountResult { sloc: 0, comments: 0, blanks: line_count(source) as u64 },
    };

    let root = tree.root_node();

    let comment_kinds: HashSet<&str> = spec.comment_kinds.iter().copied().collect();
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    collect_comment_ranges(&root, &comment_kinds, &mut comment_ranges);

    let total_lines = line_count(source);
    let mut line_is_comment = vec![false; total_lines];

    for &(start, end) in &comment_ranges {
        let start_line = byte_to_line(source, start, total_lines);
        let end_line = byte_to_line(source, end.saturating_sub(1), total_lines);
        for line in line_is_comment[start_line..=end_line].iter_mut() {
            *line = true;
        }
    }

    let mut sloc = 0u64;
    let mut comments = 0u64;
    let mut blanks = 0u64;

    for (line_idx, is_comment) in line_is_comment.iter().enumerate() {
        let line_range = line_byte_range(source, line_idx);
        let line_text = &source[line_range.0..line_range.1];
        let is_blank = line_text.iter().all(|&b| b.is_ascii_whitespace());

        if is_blank {
            blanks += 1;
        } else if *is_comment {
            let has_code = {
                let mut i = line_range.0;
                let mut found = false;
                while i < line_range.1 {
                    let in_comment = comment_ranges.iter().any(|&(cs, ce)| i >= cs && i < ce);
                    if !in_comment && !source[i].is_ascii_whitespace() {
                        found = true;
                        break;
                    }
                    i += 1;
                }
                found
            };
            if has_code {
                sloc += 1;
            } else {
                comments += 1;
            }
        } else {
            sloc += 1;
        }
    }

    CountResult { sloc, comments, blanks }
}

fn collect_comment_ranges(
    node: &Node,
    comment_kinds: &HashSet<&str>,
    ranges: &mut Vec<(usize, usize)>,
) {
    if comment_kinds.contains(node.kind()) {
        ranges.push((node.start_byte(), node.end_byte()));
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            collect_comment_ranges(&child, comment_kinds, ranges);
        }
    }
}

fn line_count(source: &[u8]) -> usize {
    if source.is_empty() {
        return 0;
    }
    source.iter().filter(|&&b| b == b'\n').count()
        + if source.last() != Some(&b'\n') { 1 } else { 0 }
}

fn byte_to_line(source: &[u8], byte: usize, total_lines: usize) -> usize {
    if byte >= source.len() {
        return total_lines.saturating_sub(1);
    }
    if source.is_empty() {
        return 0;
    }
    source[..=byte.min(source.len() - 1)]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        .min(total_lines.saturating_sub(1))
}

fn line_byte_range(source: &[u8], line_idx: usize) -> (usize, usize) {
    if source.is_empty() {
        return (0, 0);
    }
    let mut start = 0;
    let mut current_line = 0;
    for (i, &b) in source.iter().enumerate() {
        if current_line == line_idx {
            let end = source[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map(|pos| i + pos)
                .unwrap_or(source.len());
            return (i, end);
        }
        if b == b'\n' {
            current_line += 1;
            start = i + 1;
        }
    }
    if current_line == line_idx && start <= source.len() {
        return (start, source.len());
    }
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_count_empty() {
        assert_eq!(line_count(b""), 0);
    }

    #[test]
    fn test_line_count_single() {
        assert_eq!(line_count(b"hello"), 1);
    }

    #[test]
    fn test_line_count_multiple() {
        assert_eq!(line_count(b"a\nb\nc"), 3);
    }

    #[test]
    fn test_line_count_trailing_newline() {
        assert_eq!(line_count(b"a\nb\n"), 2);
    }

    #[test]
    fn test_byte_to_line() {
        let src = b"abc\ndef\nghi";
        assert_eq!(byte_to_line(src, 0, 3), 0);
        assert_eq!(byte_to_line(src, 4, 3), 1);
        assert_eq!(byte_to_line(src, 8, 3), 2);
    }

    #[test]
    fn test_line_byte_range() {
        let src = b"hello\nworld\n!";
        assert_eq!(line_byte_range(src, 0), (0, 5));
        assert_eq!(line_byte_range(src, 1), (6, 11));
        assert_eq!(line_byte_range(src, 2), (12, 13));
    }

    #[test]
    fn test_count_blank() {
        let spec = LanguageSpec {
            name: "test",
            extensions: &[],
            shebangs: &[],
            grammar_fn: tree_sitter_bash::LANGUAGE,
            comment_kinds: &["comment"],
        };
        let result = count(b"\n\n\n", &spec);
        assert_eq!(result.sloc, 0);
        assert_eq!(result.comments, 0);
        assert_eq!(result.blanks, 3);
    }
}
