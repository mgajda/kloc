use std::collections::HashSet;
use tree_sitter::{Node, Parser};
use crate::complexity::NodeCounts;
use crate::language::LanguageSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CountResult {
    pub sloc: u64,
    pub comments: u64,
    pub blanks: u64,
    pub nodes: NodeCounts,
}

impl CountResult {
    pub fn total(&self) -> u64 {
        self.sloc + self.comments + self.blanks
    }
}

pub fn count(source: &[u8], spec: &LanguageSpec) -> CountResult {
    let empty = CountResult {
        sloc: 0,
        comments: 0,
        blanks: line_count(source) as u64,
        nodes: NodeCounts::default(),
    };
    let mut parser = Parser::new();
    if parser.set_language(&spec.grammar()).is_err() {
        return empty;
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return empty,
    };

    let root = tree.root_node();

    let comment_kinds: HashSet<&str> = spec.comment_kinds.iter().copied().collect();
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    collect_comment_ranges(&root, &comment_kinds, &mut comment_ranges);

    let nodes = count_nodes(&root);

    // Build a line-start index so line lookups are O(1); a per-range rescan
    // of the whole file is O(n²) on large codebases.
    let line_starts = line_starts(source);
    let total_lines = line_starts.len();
    let mut line_is_comment = vec![false; total_lines];

    for &(start, end) in &comment_ranges {
        let start_line = line_index(&line_starts, start, total_lines);
        let end_line = line_index(&line_starts, end.saturating_sub(1), total_lines);
        for line in line_is_comment[start_line..=end_line].iter_mut() {
            *line = true;
        }
    }

    // Merge adjacent intervals; the sweep below expects disjoint, sorted ones.
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for &(s, e) in &comment_ranges {
        if let Some(last) = merged.last_mut()
            && s <= last.1 {
            if e > last.1 { last.1 = e; }
        } else {
            merged.push((s, e));
        }
    }

    let mut sloc = 0u64;
    let mut comments = 0u64;
    let mut blanks = 0u64;
    // Monotonic cursor into `merged` keeps the scan linear; a per-line scan
    // of the intervals would be O(lines × intervals).
    let mut ci = 0usize;

    for (line_idx, is_comment) in line_is_comment.iter().enumerate() {
        let line_start = line_starts[line_idx];
        let line_end = if line_idx + 1 < total_lines {
            line_starts[line_idx + 1]
        } else {
            source.len()
        };
        let line_text = &source[line_start..line_end];
        let is_blank = line_text.iter().all(|&b| b.is_ascii_whitespace());

        if is_blank {
            blanks += 1;
        } else if *is_comment {
            // Does the line contain any non-whitespace byte outside a comment?
            let mut i = line_start;
            let mut has_code = false;
            while i < line_end {
                // Skip intervals wholly before the current byte.
                while ci < merged.len() && merged[ci].1 <= i {
                    ci += 1;
                }
                // If inside a comment interval, jump to its end.
                if ci < merged.len() && i >= merged[ci].0 {
                    i = line_end.min(merged[ci].1);
                    continue;
                }
                // Outside any comment: a non-whitespace byte means code.
                if !source[i].is_ascii_whitespace() {
                    has_code = true;
                    break;
                }
                i += 1;
            }
            if has_code {
                sloc += 1;
            } else {
                comments += 1;
            }
        } else {
            sloc += 1;
        }
    }

    CountResult { sloc, comments, blanks, nodes }
}

/// Byte offsets where each line starts. `line_starts[0] = 0`; a trailing
/// newline is not a line of its own; an empty file has no lines.
fn line_starts(source: &[u8]) -> Vec<usize> {
    if source.is_empty() {
        return vec![];
    }
    let mut starts = vec![0usize];
    for (i, &b) in source.iter().enumerate() {
        if b == b'\n' && i + 1 < source.len() {
            starts.push(i + 1);
        }
    }
    starts
}

fn count_nodes(root: &Node) -> NodeCounts {
    let mut nodes = NodeCounts::default();
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if node.is_named() {
            nodes.named_nodes += 1;
        }
        if node.child_count() == 0 {
            nodes.leaf_tokens += 1;
        }
        push_children_reversed(&mut stack, &node);
    }
    nodes
}

/// Push `node`'s children onto `stack` right-to-left so they pop in
/// left-to-right order. A tree cursor is O(children); `node.child(i)` per
/// index rescans the children array from the start each time, O(children²).
fn push_children_reversed<'tree>(stack: &mut Vec<Node<'tree>>, node: &Node<'tree>) {
    let mut child = node.walk();
    if !child.goto_first_child() {
        return;
    }
    let mut children: Vec<Node> = Vec::new();
    loop {
        children.push(child.node());
        if !child.goto_next_sibling() {
            break;
        }
    }
    stack.extend(children.into_iter().rev());
}

/// Pre-order walk collecting comment ranges. Iterative so deep nesting cannot
/// overflow the call stack.
fn collect_comment_ranges(
    root: &Node,
    comment_kinds: &HashSet<&str>,
    ranges: &mut Vec<(usize, usize)>,
) {
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if comment_kinds.contains(node.kind()) {
            ranges.push((node.start_byte(), node.end_byte()));
        }
        push_children_reversed(&mut stack, &node);
    }
}

fn line_count(source: &[u8]) -> usize {
    if source.is_empty() {
        return 0;
    }
    source.iter().filter(|&&b| b == b'\n').count()
        + if source.last() != Some(&b'\n') { 1 } else { 0 }
}

/// Binary-search the line that contains `byte`, given the `line_starts` index.
fn line_index(starts: &[usize], byte: usize, total_lines: usize) -> usize {
    match starts.binary_search(&byte) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
    .min(total_lines.saturating_sub(1))
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
    fn test_line_starts() {
        assert_eq!(line_starts(b""), Vec::<usize>::new());
        assert_eq!(line_starts(b"abc\ndef\nghi"), vec![0, 4, 8]);
        // Trailing newline is not its own line.
        assert_eq!(line_starts(b"a\nb\n"), vec![0, 2]);
    }

    #[test]
    fn test_line_index() {
        let starts = line_starts(b"abc\ndef\nghi");
        let n = starts.len();
        assert_eq!(line_index(&starts, 0, n), 0);
        assert_eq!(line_index(&starts, 4, n), 1);
        assert_eq!(line_index(&starts, 8, n), 2);
        // Bytes beyond the last newline map to the final line.
        assert_eq!(line_index(&starts, 9, n), 2);
        assert_eq!(line_index(&starts, 999, n), 2);
    }

    #[test]
    fn test_count_comment_code_classification() {
        use tree_sitter::Language;
        use crate::language::LanguageCategory;
        let spec = LanguageSpec {
            name: "test",
            category: LanguageCategory::Programming,
            subgroup: None,
            extensions: &[],
            shebangs: &[],
            filenames: &[],
            grammar_fn: || Language::new(tree_sitter_rust::LANGUAGE),
            comment_kinds: &["line_comment", "block_comment"],
        };
        // Line 1: trailing comment on code → SLOC.
        // Line 2: pure comment → comments.
        // Line 3: code.
        let src = b"let x = 1; // trailing\n// pure comment\nlet y = 2;\n";
        let r = count(src, &spec);
        assert_eq!(r.sloc, 2, "trailing-comment and plain code lines are SLOC");
        assert_eq!(r.comments, 1, "pure comment line is a comment");
        assert_eq!(r.blanks, 0);
    }

    #[test]
    fn test_count_blank() {
        use tree_sitter::Language;
        use crate::language::LanguageCategory;
        let spec = LanguageSpec {
            name: "test",
            category: LanguageCategory::Programming,
            subgroup: None,
            extensions: &[],
            shebangs: &[],
            filenames: &[],
            grammar_fn: || Language::new(tree_sitter_bash::LANGUAGE),
            comment_kinds: &["comment"],
        };
        let result = count(b"\n\n\n", &spec);
        assert_eq!(result.sloc, 0);
        assert_eq!(result.comments, 0);
        assert_eq!(result.blanks, 3);
    }
}
