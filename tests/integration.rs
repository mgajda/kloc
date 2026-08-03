use std::path::{Path, PathBuf};
use std::fs;

fn write_file(dir: &Path, name: &str, content: &[u8]) {
    fs::write(dir.join(name), content).unwrap();
}

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kloc_test_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn assert_contains(report: &str, lang: &str, sloc: u64) {
    // The per-language row is "Name<12><loc right-aligned to 10>".
    let pattern = format!("{lang:12}{sloc:>10}");
    assert!(
        report.contains(&pattern),
        "Expected '{}' in report.\nFull report:\n{}",
        pattern.trim(), report
    );
}

fn default_filter() -> kloc::LanguageFilter {
    kloc::LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    }
}

fn test_opts() -> kloc::RunOptions {
    kloc::RunOptions {
        sloc_only: false,
        cache: kloc::cache::Cache::new(false),
        ignore: kloc::walker::DirIgnore::new(false),
    }
}

fn test_colors() -> kloc::color::Colors {
    kloc::color::Colors::from_mode(kloc::color::ColorMode::Never)
}

fn test_ai_config() -> kloc::ai_config::AiConfig {
    kloc::ai_config::default_config()
}

fn run_and_get_text(paths: &[PathBuf]) -> String {
    let filter = default_filter();
    let report = kloc::run(paths, &filter, &test_opts());
    kloc::output::format(&report, &kloc::output::OutputFormat::Text, true, test_colors(), &test_ai_config(), None)
}

fn run_and_get_json(paths: &[PathBuf]) -> serde_json::Value {
    let filter = default_filter();
    let report = kloc::run(paths, &filter, &test_opts());
    let json = kloc::output::format(&report, &kloc::output::OutputFormat::Json, true, test_colors(), &test_ai_config(), None);
    serde_json::from_str(&json).unwrap()
}

#[test]
fn integration_single_language_rust() {
    let dir = test_dir("single_rust");
    write_file(&dir, "main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Rust", 3);
}

fn has_language(name: &str) -> bool {
    let reg = kloc::language::registry();
    reg.languages().iter().any(|l| l.name == name)
}

#[test]
fn integration_two_languages_rust_and_c() {
    let dir = test_dir("two_langs");
    write_file(&dir, "main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    write_file(&dir, "helper.c", b"int helper() {\n    return 42;\n}\n");

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Rust", 3);
    assert_contains(&text, "C", 3);
}

#[test]
fn integration_multi_language() {
    let dir = test_dir("multi_lang");
    write_file(&dir, "main.rs", b"fn main() {}\n");
    write_file(&dir, "main.py", b"def main():\n    pass\n");
    write_file(&dir, "main.c", b"int main() {\n    return 0;\n}\n");
    write_file(&dir, "app.js", b"function main() {\n    return 1;\n}\n");
    write_file(&dir, "run.sh", b"#!/bin/sh\necho hi\n");

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Rust", 1);
    assert_contains(&text, "Python", 2);
    assert_contains(&text, "C", 3);
    assert_contains(&text, "JavaScript", 3);
    assert_contains(&text, "Bash", 1);
}

#[test]
fn integration_multi_language_detailed() {
    if !has_language("Haskell") || !has_language("Go") || !has_language("Java") {
        return;
    }
    let dir = test_dir("multi_detailed");
    write_file(&dir, "main.rs", b"fn main() {\n    println!(\"hi\");\n}\n");
    write_file(&dir, "Main.hs", b"module Main where\n\nmain :: IO ()\nmain = putStrLn \"hi\"\n");
    write_file(&dir, "hello.go", b"package main\nfunc main() {}\n");
    write_file(&dir, "Main.java", b"public class Main {\n    public static void main(String[] args) {}\n}\n");

    let json = run_and_get_json(&[dir]);
    let langs = json["by_language"].as_array().unwrap();
    let total_sloc = json["total_sloc"].as_u64().unwrap();

    let mut map = std::collections::HashMap::new();
    for entry in langs {
        let name = entry["name"].as_str().unwrap().to_string();
        let sloc = entry["sloc"].as_u64().unwrap();
        let files = entry["files"].as_u64().unwrap();
        map.insert(name, (sloc, files));
    }

    assert_eq!(map.get("Rust"), Some(&(3, 1)));
    assert_eq!(map.get("Haskell"), Some(&(3, 1)));
    assert_eq!(map.get("Go"), Some(&(2, 1)));
    assert_eq!(map.get("Java"), Some(&(3, 1)));

    let sum: u64 = map.values().map(|(s, _)| s).sum();
    assert_eq!(total_sloc, sum, "total should match sum of per-language SLOC");
}

#[test]
fn integration_with_makefile() {
    if !has_language("Make") { return; }
    let dir = test_dir("with_makefile");
    write_file(&dir, "Makefile", b"all:\n\techo hello\n");
    write_file(&dir, "main.c", b"int main() { return 0; }\n");

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Make", 2);
    assert_contains(&text, "C", 1);
}

#[test]
fn integration_with_shebang() {
    let dir = test_dir("shebang_scripts");
    let script_py = dir.join("build");
    fs::write(&script_py, b"#!/usr/bin/env python3\nprint('hi')\n").unwrap();
    let script_sh = dir.join("run");
    fs::write(&script_sh, b"#!/bin/sh\necho hi\n").unwrap();

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Python", 1);
    assert_contains(&text, "Bash", 1);
}

#[test]
fn integration_multiple_directories() {
    let dir1 = test_dir("proj1");
    write_file(&dir1, "lib.rs", b"pub fn hello() {}\n");

    let dir2 = test_dir("proj2");
    write_file(&dir2, "lib.py", b"def hello():\n    pass\n");

    let text = run_and_get_text(&[dir1, dir2]);
    assert_contains(&text, "Rust", 1);
    assert_contains(&text, "Python", 2);
}

#[test]
fn integration_html_and_css() {
    if !has_language("HTML") || !has_language("CSS") { return; }
    let dir = test_dir("web_mix");
    write_file(&dir, "index.html", b"<html>\n<body>\n<p>hi</p>\n</body>\n</html>\n");
    write_file(&dir, "style.css", b"body {\n    color: red;\n}\n");
    write_file(&dir, "app.js", b"console.log('hi');\n");

    let json = run_and_get_json(&[dir]);
    let mut map = std::collections::HashMap::new();
    for entry in json["by_language"].as_array().unwrap() {
        map.insert(entry["name"].as_str().unwrap().to_string(), entry["sloc"].as_u64().unwrap());
    }
    assert_eq!(map.get("HTML"), Some(&5), "HTML should be detected if feature enabled");
    assert_eq!(map.get("CSS"), Some(&3), "CSS should be detected if feature enabled");
    assert_eq!(map.get("JavaScript"), Some(&1), "JS should be detected");
}

#[test]
fn integration_blank_and_comment_only_files() {
    let dir = test_dir("blank_comment");
    write_file(&dir, "main.rs", b"// just a comment\n");
    write_file(&dir, "empty.rs", b"");

    let text = run_and_get_text(&[dir]);
    assert_contains(&text, "Rust", 0);
}

#[test]
fn integration_sloc_count_equals_sum() {
    let dir = test_dir("sum_check");
    write_file(&dir, "a.rs", b"fn a() {}\n");
    write_file(&dir, "b.rs", b"fn b() {}\n");
    write_file(&dir, "c.py", b"def c():\n    pass\n");

    let json = run_and_get_json(&[dir]);
    let total = json["total_sloc"].as_u64().unwrap();
    let sum: u64 = json["by_language"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["sloc"].as_u64().unwrap())
        .sum();
    assert_eq!(total, sum, "total_sloc must equal sum of per-language SLOC");
}

#[test]
fn integration_filter_only_programming() {
    let dir = test_dir("filter_prog");
    write_file(&dir, "main.rs", b"fn main() {}\n");

    let filter = kloc::LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: true,
        only_machine: false,
    };
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
    assert_eq!(report.total_sloc, 1, "only Rust should be counted (1 sloc)");
    assert_eq!(report.by_language.len(), 1, "only Rust in results");
    assert_eq!(report.by_language[0].name, "Rust");
}

#[test]
fn integration_filter_only_machine() {
    if !has_language("JSON") { return; }
    let dir = test_dir("filter_machine");
    write_file(&dir, "main.rs", b"fn main() {}\n");
    write_file(&dir, "data.json", b"{\"key\": 1}\n");

    let filter = kloc::LanguageFilter {
        only: vec![],
        exclude: vec![],
        only_programming: false,
        only_machine: true,
    };
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
    assert!(report.total_sloc > 0, "machine languages should have sloc");
    assert!(report.by_language.iter().any(|l| l.name == "JSON"), "JSON should be included");
    assert!(report.by_language.iter().all(|l| l.name != "Rust"), "Rust should be excluded");
}

#[test]
fn integration_filter_only_specific_languages() {
    let dir = test_dir("filter_only");
    write_file(&dir, "main.rs", b"fn main() {}\n");
    write_file(&dir, "main.py", b"def main():\n    pass\n");
    write_file(&dir, "app.js", b"function main() {}\n");

    let filter = kloc::LanguageFilter {
        only: vec!["python".to_string()],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
    assert_eq!(report.total_sloc, 2, "only Python should be counted (2 sloc)");
    assert_eq!(report.by_language.len(), 1);
    assert_eq!(report.by_language[0].name, "Python");
}

#[test]
fn integration_filter_exclude_languages() {
    let dir = test_dir("filter_exclude");
    write_file(&dir, "main.rs", b"fn main() {}\n");
    write_file(&dir, "main.py", b"def main():\n    pass\n");

    let filter = kloc::LanguageFilter {
        only: vec![],
        exclude: vec!["python".to_string()],
        only_programming: false,
        only_machine: false,
    };
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
    assert_eq!(report.total_sloc, 1, "only Rust should be counted");
    assert_eq!(report.by_language.len(), 1);
    assert_eq!(report.by_language[0].name, "Rust");
}

#[test]
fn integration_filter_only_programming_and_only() {
    let dir = test_dir("filter_prog_only");
    write_file(&dir, "main.rs", b"fn main() {}\n");
    write_file(&dir, "data.json", b"{\"key\": 1}\n");

    let filter = kloc::LanguageFilter {
        only: vec!["rust".to_string()],
        exclude: vec![],
        only_programming: false,
        only_machine: false,
    };
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
    assert_eq!(report.total_sloc, 1, "only Rust should be counted");
    assert_eq!(report.by_language[0].name, "Rust");
}

#[test]
fn integration_json_parseable() {    let dir = test_dir("json_parse");
    write_file(&dir, "main.rs", b"fn main() {}\n");

    let json_str = {
        let filter = default_filter();
        let report = kloc::run(std::slice::from_ref(&dir), &filter, &test_opts());
        kloc::output::format(&report, &kloc::output::OutputFormat::Json, true, test_colors(), &test_ai_config(), None)
    };

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("JSON output must be valid");
    assert!(parsed["by_language"].is_array());
    assert!(parsed["total_sloc"].is_u64());
    assert!(parsed["total_files"].is_u64());
}

// ---- Directory-ignore tests ----------------------------------------------

#[test]
fn integration_ignore_default_dirs() {
    let dir = test_dir("ignore_defaults");
    // Some ignored dirs and one real file.
    for sub in ["node_modules", ".git", "dist", "__pycache__", "src"] {
        fs::create_dir_all(dir.join(sub)).unwrap();
    }
    write_file(&dir, "src/main.rs", b"fn main() {}\n");
    write_file(&dir, "node_modules/lib.rs", b"fn node() {}\n");
    write_file(&dir, ".git/x.rs", b"fn git() {}\n");
    write_file(&dir, "dist/y.rs", b"fn dist() {}\n");

    let filter = default_filter();
    let mut opts = test_opts();
    opts.ignore = kloc::walker::DirIgnore::new(true);
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &opts);
    // Only src/main.rs is counted (default ignores exclude the others).
    assert_eq!(report.total_files, 1, "only src/main.rs counted");
    assert_eq!(report.total_sloc, 1);
}

#[test]
fn integration_ignore_custom_and_no_ignore() {
    let dir = test_dir("ignore_custom");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("gen")).unwrap();
    write_file(&dir, "src/main.rs", b"fn main() {}\n");
    write_file(&dir, "gen/gen.rs", b"fn gen() {}\n");

    let filter = default_filter();

    // Defaults on: "src" is not a default-ignored dir, so both are counted
    // unless we add "gen".
    let mut opts = test_opts();
    opts.ignore = kloc::walker::DirIgnore::new(true);
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &opts);
    assert_eq!(report.total_files, 2, "defaults don't ignore src or gen");

    // Add "gen" to ignores → only src counted.
    opts.ignore.add("gen");
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &opts);
    assert_eq!(report.total_files, 1, "gen ignored after add");

    // Remove "node_modules" (a default) and disable defaults for gen check.
    let mut opts = test_opts();
    opts.ignore = kloc::walker::DirIgnore::new(false);
    let report = kloc::run(std::slice::from_ref(&dir), &filter, &opts);
    assert_eq!(report.total_files, 2, "no defaults, both counted");
}

// ---- Git-history consistency tests ---------------------------------------
//
// These compare --history against source-tree metrics on randomly generated
// tree-sitter-parseable Rust code. Per the seed SOP (see AGENTS.md), the RNG
// is seeded from system entropy each run and the chosen seed is printed, so a
// failure can be reproduced deterministically with KLOK_TEST_SEED=<seed>.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

/// Seed the RNG: honour `KLOK_TEST_SEED` if set, else derive from wall-clock
/// entropy. Prints the chosen seed so failures are reproducible.
fn test_rng() -> (u64, rand::rngs::StdRng) {
    let seed: u64 = match std::env::var("KLOK_TEST_SEED") {
        Ok(s) => s.parse().expect("KLOK_TEST_SEED must be an integer"),
        Err(_) => {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
                .expect("clock before epoch").as_nanos();
            (nanos ^ (nanos >> 32)) as u64
        }
    };
    println!("KLOK_TEST_SEED={seed}");
    use rand::SeedableRng;
    (seed, rand::rngs::StdRng::seed_from_u64(seed))
}

/// Generate a small tree-sitter-parseable Rust expression.
fn gen_expr(rng: &mut impl rand::Rng, depth: u32) -> String {
    if depth == 0 {
        let int = rng.gen_range(0..1000u32);
        return int.to_string();
    }
    let name = |i: u8| format!("v{i}");
    match rng.gen_range(0..8) {
        0 => rng.gen_range(0..1000u32).to_string(),
        1 => format!("{} + {}", gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        2 => format!("{} * {}", gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        3 => format!("{} - {}", gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        4 => name(rng.gen_range(0..5)),
        5 => format!("{}.wrapping_add({})", name(rng.gen_range(0..5)), gen_expr(rng, depth - 1)),
        6 => format!("if {} > 0 {{ {} }} else {{ {} }}",
            gen_expr(rng, depth - 1), gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        _ => format!("({})", gen_expr(rng, depth - 1)),
    }
}

/// Generate one tree-sitter-parseable Rust function with random statements.
fn gen_fn(rng: &mut impl rand::Rng, idx: usize) -> String {
    let mut body = String::new();
    let n = rng.gen_range(1..4);
    for _ in 0..n {
        match rng.gen_range(0..5) {
            0 => body.push_str(&format!("    let v{} = {};\n", rng.gen_range(0..5), gen_expr(rng, 2))),
            1 => body.push_str(&format!("    println!(\"{}\", {});\n", "x", gen_expr(rng, 2))),
            2 => body.push_str(&format!("    if {} < 10 {{ {}; }}\n", gen_expr(rng, 1), gen_expr(rng, 1))),
            3 => body.push_str(&format!("    for v{} in 0..{} {{ {}; }}\n",
                rng.gen_range(0..5), rng.gen_range(1..20), gen_expr(rng, 1))),
            _ => body.push_str(&format!("    let v{} = v{} + 1;\n", rng.gen_range(0..5), rng.gen_range(0..5))),
        }
    }
    format!("pub fn f{idx}() -> u32 {{\n{body}    1\n}}\n")
}

/// Generate `n` parseable Rust functions joined together.
fn gen_rust(rng: &mut impl rand::Rng, n: usize) -> String {
    (0..n).map(|i| gen_fn(rng, i)).collect::<String>()
}

/// Initialise a git repo in `dir` (quietly, using a local identity).
fn git_init(dir: &std::path::Path) {
    Command::new("git").args(["init", "-q"]).current_dir(dir).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir).output().unwrap();
    Command::new("git")
        .args(["config", "user.name", "test"])
        .current_dir(dir).output().unwrap();
    Command::new("git").args(["add", "-A"]).current_dir(dir).output().unwrap();
}

fn git_commit(dir: &std::path::Path, msg: &str) {
    Command::new("git")
        .args(["commit", "-q", "-m", msg])
        .current_dir(dir).output().unwrap();
}

/// Run history analysis and return the (report, formatted text).
fn run_history(dir: &std::path::Path) -> (kloc::history::HistoryReport, String) {
    let filter = default_filter();
    let cfg = kloc::ai_config::default_config();
    let report = kloc::history::run_history(
        &[dir.to_path_buf()], &filter, None, None, &cfg, None,
    ).expect("history should run");
    let text = kloc::output::format_history(&report, test_colors(), &test_ai_config(), None);
    (report, text)
}

/// Run source-tree analysis and return the report.
fn run_source(dir: &std::path::Path) -> kloc::Report {
    let filter = default_filter();
    kloc::run(&[dir.to_path_buf()], &filter, &test_opts())
}

/// A file added at creation in one commit. With randomly generated parseable
/// code, history metrics must match the source-tree metrics for the same file.
#[test]
fn integration_history_single_file_creation_matches_source() {
    if Command::new("git").arg("--version").output().is_err() { return; }
    let (seed, mut rng) = test_rng();
    let n_fn = rng.gen_range(3..12);
    let code = gen_rust(&mut rng, n_fn);

    let dir = test_dir("hist_single_creation");
    write_file(&dir, "lib.rs", code.as_bytes());
    git_init(&dir);
    git_commit(&dir, "add lib.rs");

    let src = run_source(&dir);
    let (hist, hist_text) = run_history(&dir);

    assert_eq!(hist.commits, 1, "one commit (seed={seed})");
    assert!(hist_text.contains("Schedule (from diffs)"), "history must show schedule table (seed={seed})");
    assert!(hist_text.contains("Halstead"), "history must show Halstead column (seed={seed})");
    assert!(hist.halstead.is_some(), "history must compute Halstead (seed={seed})");
    assert_eq!(hist.total_added_lines, src.total_sloc,
        "history added LOC must equal source SLOC (seed={seed})");
    assert_eq!(hist.total_removed_lines, 0, "no removals (seed={seed})");
}

/// A file built by adding one random function per commit (no removals, no
/// modifications). History and source-tree modes must report the same final
/// metrics.
#[test]
fn integration_history_single_function_additions_match_source() {
    if Command::new("git").arg("--version").output().is_err() { return; }
    let (seed, mut rng) = test_rng();
    let n_fns = rng.gen_range(3..8);

    let dir = test_dir("hist_fn_additions");
    // Commit 1: first function.
    write_file(&dir, "lib.rs", gen_fn(&mut rng, 0).as_bytes());
    git_init(&dir);
    git_commit(&dir, "add fn f0");

    // Append one function per commit.
    for i in 1..n_fns {
        let content = std::fs::read_to_string(dir.join("lib.rs")).unwrap();
        let extra = gen_fn(&mut rng, i as usize);
        std::fs::write(dir.join("lib.rs"), content + &extra).unwrap();
        Command::new("git").args(["add", "lib.rs"]).current_dir(&dir).output().unwrap();
        git_commit(&dir, &format!("add fn f{i}"));
    }

    let src = run_source(&dir);
    let (hist, _hist_text) = run_history(&dir);

    assert_eq!(hist.commits, n_fns, "one commit per function (seed={seed})");
    assert_eq!(hist.total_added_lines, src.total_sloc,
        "history added LOC must equal final source SLOC, no removals (seed={seed})");
    assert_eq!(hist.total_removed_lines, 0, "no removals (seed={seed})");
    assert!(hist.halstead.is_some(), "Halstead must be computed (seed={seed})");
    assert!(src.nodes.leaf_tokens > 0, "source must have leaf tokens (seed={seed})");
}

/// Deeply nested (but tiny) source must complete without overflowing the stack
/// or going super-linear. The old recursive complexity walk overflowed the
/// call stack around ~16k nesting depth and was quadratic (`node.parent()`
/// walks down from the tree root for every node); at 40k depth it crashed, and
/// the quadratic version took minutes on inputs a tenth this deep.
#[test]
fn integration_deep_nesting_completes_and_counts() {
    let depth = 40_000;
    let mut code = String::from("pub fn deep() -> u64 {\n    ");
    code.push('1');
    for _ in 0..depth {
        code.push_str(" + (1");
    }
    for _ in 0..depth {
        code.push(')');
    }
    code.push_str("\n}\n");

    let dir = test_dir("deep_nesting");
    write_file(&dir, "main.rs", code.as_bytes());
    let json = run_and_get_json(&[dir]);
    // The regression check is completing at all (the old code crashed here).
    assert!(json["total_sloc"].as_u64().unwrap() > 0, "deeply nested source must parse to non-zero SLOC");
    assert!(json["total_files"].as_u64().unwrap() == 1, "exactly one file");
}

/// Very wide (many top-level children) source must count correctly. The
/// `count_nodes` / `collect_comment_ranges` walks once used
/// `node.child(i)` per index, which rescans the children array from the
/// start for each `i` — O(k²) on a node with k children, e.g. the root of a
/// flat file. The walks now enumerate children with a tree cursor (O(k)).
#[test]
fn integration_wide_nesting_counts_every_function() {
    let functions = 30_000;
    let mut code = String::new();
    for i in 0..functions {
        code.push_str(&format!("fn f{i}() -> u64 {{ {i} }}\n"));
    }

    let dir = test_dir("wide_nesting");
    write_file(&dir, "main.rs", code.as_bytes());
    let json = run_and_get_json(&[dir]);
    assert_eq!(json["total_files"].as_u64().unwrap(), 1, "exactly one file");
    assert_eq!(
        json["mccabe"]["function_count"].as_u64().unwrap(),
        functions as u64,
        "every top-level function must be counted"
    );
    assert!(
        json["nodes"]["named_nodes"].as_u64().unwrap() >= functions as u64,
        "each function contributes named nodes"
    );
}

