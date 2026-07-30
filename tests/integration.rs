use std::path::PathBuf;
use std::fs;

fn write_file(dir: &PathBuf, name: &str, content: &[u8]) {
    fs::write(dir.join(name), content).unwrap();
}

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kloc_test_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn assert_contains(report: &str, lang: &str, sloc: u64) {
    let pattern = format!("{lang:12} {sloc:>8}");
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

fn run_and_get_text(paths: &[PathBuf]) -> String {
    let filter = default_filter();
    let report = kloc::run(paths, &filter);
    kloc::output::format(&report, &kloc::output::OutputFormat::Text)
}

fn run_and_get_json(paths: &[PathBuf]) -> serde_json::Value {
    let filter = default_filter();
    let report = kloc::run(paths, &filter);
    let json = kloc::output::format(&report, &kloc::output::OutputFormat::Json);
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
    let report = kloc::run(&[dir], &filter);
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
    let report = kloc::run(&[dir], &filter);
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
    let report = kloc::run(&[dir], &filter);
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
    let report = kloc::run(&[dir], &filter);
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
    let report = kloc::run(&[dir], &filter);
    assert_eq!(report.total_sloc, 1, "only Rust should be counted");
    assert_eq!(report.by_language[0].name, "Rust");
}

#[test]
fn integration_json_parseable() {
    let dir = test_dir("json_parse");
    write_file(&dir, "main.rs", b"fn main() {}\n");

    let json_str = {
        let filter = default_filter();
        let report = kloc::run(&[dir], &filter);
        kloc::output::format(&report, &kloc::output::OutputFormat::Json)
    };

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("JSON output must be valid");
    assert!(parsed["by_language"].is_array());
    assert!(parsed["total_sloc"].is_u64());
    assert!(parsed["total_files"].is_u64());
}
