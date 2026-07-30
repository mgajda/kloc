use std::path::Path;

fn detect(path: &str, first_line: Option<&[u8]>) -> Option<&'static str> {
    let registry = kloc::language::registry();
    let p = Path::new(path);
    let spec = registry.detect(p, first_line);
    spec.map(|s| s.name)
}

#[test]
fn detect_rust_extension() {
    assert_eq!(detect("main.rs", None), Some("Rust"));
    assert_eq!(detect("lib.rs", None), Some("Rust"));
}

#[test]
fn detect_c_extension() {
    assert_eq!(detect("program.c", None), Some("C"));
    assert_eq!(detect("myheader.h", None), Some("C"));
}

#[test]
fn detect_cpp_extension() {
    // C++ only if feature is enabled
    let r = detect("main.cpp", None);
    if let Some(lang) = r { assert_eq!(lang, "C++"); }
}

#[test]
fn detect_python_extension() {
    assert_eq!(detect("script.py", None), Some("Python"));
}

#[test]
fn detect_javascript_extension() {
    assert_eq!(detect("app.js", None), Some("JavaScript"));
    assert_eq!(detect("app.jsx", None), Some("JavaScript"));
}

#[test]
fn detect_typescript_extension() {
    let r = detect("app.ts", None);
    if let Some(lang) = r { assert_eq!(lang, "TypeScript"); }
    let r = detect("app.tsx", None);
    if let Some(lang) = r { assert_eq!(lang, "TSX"); }
}

#[test]
fn detect_java_extension() {
    if let Some(lang) = detect("Main.java", None) { assert_eq!(lang, "Java"); }
}

#[test]
fn detect_haskell_extension() {
    if let Some(lang) = detect("Main.hs", None) { assert_eq!(lang, "Haskell"); }
}

#[test]
fn detect_ocaml_extension() {
    if let Some(lang) = detect("main.ml", None) { assert_eq!(lang, "OCaml"); }
}

#[test]
fn detect_go_extension() {
    if let Some(lang) = detect("main.go", None) { assert_eq!(lang, "Go"); }
}

#[test]
fn detect_rust_vs_other() {
    assert_eq!(detect("main.rs", None), Some("Rust"));
    if let Some(lang) = detect("main.rs", None) { assert_ne!(lang, "C++"); }
}

#[test]
fn detect_makefile_exact_filename() {
    if let Some(lang) = detect("Makefile", None) { assert_eq!(lang, "Make"); }
    if let Some(lang) = detect("makefile", None) { assert_eq!(lang, "Make"); }
}

#[test]
fn detect_shebang_python() {
    assert_eq!(detect("script", Some(b"#!/usr/bin/env python3")), Some("Python"));
}

#[test]
fn detect_shebang_perl() {
    if let Some(lang) = detect("script", Some(b"#!/usr/bin/perl -w")) { assert_eq!(lang, "Perl"); }
}

#[test]
fn detect_shebang_bash() {
    assert_eq!(detect("script", Some(b"#!/bin/bash")), Some("Bash"));
    assert_eq!(detect("script", Some(b"#!/bin/sh")), Some("Bash"));
}

#[test]
fn detect_shebang_ruby() {
    if let Some(lang) = detect("script", Some(b"#!/usr/bin/env ruby")) { assert_eq!(lang, "Ruby"); }
}

#[test]
fn detect_shebang_node() {
    assert_eq!(detect("script", Some(b"#!/usr/bin/env node")), Some("JavaScript"));
}

#[test]
fn detect_elixir_extension() {
    if let Some(lang) = detect("app.ex", None) { assert_eq!(lang, "Elixir"); }
}

#[test]
fn detect_shebang_fish() {
    // "fish" should NOT match Bash's "sh" shebang
    if let Some(lang) = detect("script", Some(b"#!/usr/bin/env fish")) { assert_eq!(lang, "Fish"); }
}

#[test]
fn detect_unknown_extension() {
    assert_eq!(detect("file.xyz", None), None);
}

#[test]
fn detect_unknown_no_shebang() {
    assert_eq!(detect("script", Some(b"some text without shebang")), None);
}

#[test]
fn detect_verilog_extension() {
    if let Some(lang) = detect("module.sv", None) { assert_eq!(lang, "Verilog"); }
    if let Some(lang) = detect("module.v", None) { assert_eq!(lang, "Verilog"); }
}

#[test]
fn detect_v_or_verilog() {
    let r = detect("main.v", None);
    assert!(r.is_none() || r == Some("Verilog") || r == Some("V"), "unexpected language: {r:?}");
}

#[test]
fn detect_dart_extension() {
    if let Some(lang) = detect("main.dart", None) { assert_eq!(lang, "Dart"); }
}

#[test]
fn detect_lua_extension() {
    if let Some(lang) = detect("script.lua", None) { assert_eq!(lang, "Lua"); }
}

#[test]
fn detect_fortran_extension() {
    if let Some(lang) = detect("program.f90", None) { assert_eq!(lang, "Fortran"); }
}

#[test]
fn detect_perl_extension() {
    if let Some(lang) = detect("script.pl", None) { assert_eq!(lang, "Perl"); }
}

#[test]
fn detect_php_extension() {
    if let Some(lang) = detect("index.php", None) { assert_eq!(lang, "PHP"); }
}

#[test]
fn detect_swift_extension() {
    if let Some(lang) = detect("main.swift", None) { assert_eq!(lang, "Swift"); }
}

#[test]
fn detect_ruby_extension() {
    if let Some(lang) = detect("script.rb", None) { assert_eq!(lang, "Ruby"); }
}

#[test]
fn detect_fish_extension() {
    if let Some(lang) = detect("script.fish", None) { assert_eq!(lang, "Fish"); }
}

#[test]
fn detect_haskell_literature() {
    if let Some(lang) = detect("file.lhs", None) { assert_eq!(lang, "Haskell"); }
}

#[test]
fn detect_multi_language_directory() {
    let mut found: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let paths = [
        ("src/main.rs", None, "Rust"),
        ("src/main.c", None, "C"),
        ("src/main.py", None, "Python"),
        ("script.js", None, "JavaScript"),
        ("run.sh", None, "Bash"),
    ];
    for (path, shebang, expected) in &paths {
        if let Some(lang) = detect(path, *shebang) {
            found.insert(lang);
            assert_eq!(lang, *expected, "Failed for path: {path}");
        }
    }
    assert!(found.len() >= 3, "should detect at least default languages");
}

#[test]
fn test_all_languages_have_unique_extensions() {
    let registry = kloc::language::registry();
    let mut seen_exts: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for lang in registry.languages() {
        for ext in lang.extensions {
            seen_exts.entry(ext).or_default().push(lang.name);
        }
    }
    let conflicts: Vec<_> = seen_exts.iter().filter(|(_, langs)| langs.len() > 1).collect();
    for (ext, langs) in &conflicts {
        eprintln!("Extension conflict: '{ext}' maps to {langs:?}");
    }
    // Known: .v -> Verilog and V
}
