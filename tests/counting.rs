use std::path::Path;

fn count_source(source: &[u8], path: &str) -> Option<kloc::counter::CountResult> {
    let registry = kloc::language::registry();
    let spec = registry.detect(Path::new(path), None)?;
    Some(kloc::counter::count(source, spec))
}

// Default features (always available)
#[test]
fn count_rust_hello() {
    if let Some(r) = count_source(b"fn main() {\n    println!(\"hello\");\n}\n", "main.rs") { assert_eq!(r.sloc, 3); }
}

#[test]
fn count_rust_with_comments() {
    if let Some(r) = count_source(b"// comment line\nfn main() {\n    // inline\n    println!(\"hi\");\n}\n", "main.rs") {
        assert_eq!(r.sloc, 3); assert_eq!(r.comments, 2);
    }
}

#[test]
fn count_rust_block_comment() {
    if let Some(r) = count_source(b"/* block\ncomment\n*/\nfn main() {}\n", "main.rs") {
        assert_eq!(r.sloc, 1); assert_eq!(r.comments, 3);
    }
}

#[test]
fn count_rust_blank_lines() {
    if let Some(r) = count_source(b"fn a() {}\n\nfn b() {}\n", "main.rs") {
        assert_eq!(r.sloc, 2); assert_eq!(r.blanks, 1);
    }
}

#[test]
fn count_c_simple() {
    if let Some(r) = count_source(b"int main() {\n    return 0;\n}\n", "main.c") { assert_eq!(r.sloc, 3); }
}

#[test]
fn count_c_comment_only_line_and_inline() {
    if let Some(r) = count_source(b"/* header */\nint main() {\n    // inline only\n    return 0;\n}\n", "main.c") {
        assert_eq!(r.sloc, 3); assert_eq!(r.comments, 2);
    }
}

#[test]
fn count_python_simple() {
    if let Some(r) = count_source(b"def main():\n    pass\n", "main.py") { assert_eq!(r.sloc, 2); }
}

#[test]
fn count_python_with_comments() {
    if let Some(r) = count_source(b"# this is a comment\ndef main():\n    # inline\n    pass\n", "main.py") {
        assert_eq!(r.sloc, 2); assert_eq!(r.comments, 2);
    }
}

#[test]
fn count_javascript_simple() {
    if let Some(r) = count_source(b"function main() {\n    return 1;\n}\n", "app.js") { assert_eq!(r.sloc, 3); }
}

#[test]
fn count_bash_simple() {
    // shebang is a comment in tree-sitter-bash
    if let Some(r) = count_source(b"#!/bin/sh\necho hi\n", "run.sh") { assert_eq!(r.sloc, 1); assert_eq!(r.comments, 1); }
}

#[test]
fn count_empty_file() {
    if let Some(r) = count_source(b"", "empty.rs") { assert_eq!(r.sloc, 0); assert_eq!(r.comments, 0); assert_eq!(r.blanks, 0); }
}

#[test]
fn count_only_blanks() {
    if let Some(r) = count_source(b"\n\n\n", "blank.rs") { assert_eq!(r.sloc, 0); assert_eq!(r.blanks, 3); }
}

#[test]
fn count_only_comment_lines() {
    if let Some(r) = count_source(b"// comment 1\n// comment 2\n// comment 3\n", "file.rs") {
        assert_eq!(r.sloc, 0); assert_eq!(r.comments, 3);
    }
}

#[test]
fn count_inline_comment_with_code() {
    if let Some(r) = count_source(b"x = 1; // inline\n", "file.rs") { assert_eq!(r.sloc, 1); assert_eq!(r.comments, 0); }
}

#[test]
fn count_inline_comment_only_whitespace() {
    if let Some(r) = count_source(b"    // just a comment\n", "file.rs") { assert_eq!(r.sloc, 0); assert_eq!(r.comments, 1); }
}

#[test]
fn count_total_lines_is_sloc_plus_comments_plus_blanks() {
    let source = b"// header\nfn main() {\n    // inline\n    println!(\"hello\");\n}\n\n";
    if let Some(result) = count_source(source, "main.rs") {
        assert_eq!(result.sloc + result.comments + result.blanks, 6);
    }
}

// Feature-gated languages (test only if compiled in)
#[cfg(feature = "go")]
#[test]
fn count_go_simple() {
    let result = count_source(b"package main\nfunc main() {}\n", "main.go").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "go")]
#[test]
fn count_go_with_comment() {
    let result = count_source(b"// Package comment\npackage main\n\nfunc main() {}\n", "main.go").unwrap();
    assert_eq!(result.sloc, 2);
    assert_eq!(result.comments, 1);
    assert_eq!(result.blanks, 1);
}

#[cfg(feature = "haskell")]
#[test]
fn count_haskell_simple() {
    let result = count_source(b"module Main where\nmain :: IO ()\nmain = putStrLn \"hi\"\n", "Main.hs").unwrap();
    assert_eq!(result.sloc, 3);
}

#[cfg(feature = "haskell")]
#[test]
fn count_haskell_with_comment() {
    let result = count_source(b"-- Module comment\nmodule Main where\n\nmain :: IO ()\nmain = putStrLn \"hi\"\n", "Main.hs").unwrap();
    assert_eq!(result.sloc, 3);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "java")]
#[test]
fn count_java_simple() {
    let result = count_source(b"public class Main {\n    public static void main(String[] args) {}\n}\n", "Main.java").unwrap();
    assert_eq!(result.sloc, 3);
}

#[cfg(feature = "java")]
#[test]
fn count_java_with_comments() {
    let result = count_source(b"// comment\npublic class Main {\n    /* block */\n    public static void main(String[] args) {}\n}\n", "Main.java").unwrap();
    // `/* block */` on its own line (with only whitespace prefix) is a comment line
    assert_eq!(result.sloc, 3);
    assert_eq!(result.comments, 2);
}

#[cfg(feature = "ruby")]
#[test]
fn count_ruby_simple() {
    let result = count_source(b"def main\n  puts 'hi'\nend\n", "main.rb").unwrap();
    assert_eq!(result.sloc, 3);
}

#[cfg(feature = "ruby")]
#[test]
fn count_ruby_with_comment() {
    let result = count_source(b"# comment\ndef main\n  puts 'hi'\nend\n", "main.rb").unwrap();
    assert_eq!(result.sloc, 3);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "perl")]
#[test]
fn count_perl_simple() {
    let result = count_source(b"#!/usr/bin/perl\nuse strict;\nprint \"hi\\n\";\n", "script.pl").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "scala")]
#[test]
fn count_scala_simple() {
    let result = count_source(b"object Main {\n  def main(args: Array[String]): Unit = {}\n}\n", "Main.scala").unwrap();
    assert_eq!(result.sloc, 3);
}

#[cfg(feature = "ocaml")]
#[test]
fn count_ocaml_simple() {
    let result = count_source(b"let () = print_endline \"hi\"\n", "main.ml").unwrap();
    assert_eq!(result.sloc, 1);
}

#[cfg(feature = "ocaml")]
#[test]
fn count_ocaml_with_comment() {
    let result = count_source(b"(* ocaml comment *)\nlet () = print_endline \"hi\"\n", "main.ml").unwrap();
    assert_eq!(result.sloc, 1);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "elm")]
#[test]
fn count_elm_simple() {
    let result = count_source(b"module Main exposing (..)\nmain = text \"hi\"\n", "Main.elm").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "zig")]
#[test]
fn count_zig_simple() {
    let result = count_source(b"const std = @import(\"std\");\npub fn main() void {}\n", "main.zig").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "typescript")]
#[test]
fn count_typescript_simple() {
    let result = count_source(b"const x: number = 1;\n", "app.ts").unwrap();
    assert_eq!(result.sloc, 1);
}

#[cfg(feature = "elixir")]
#[test]
fn count_elixir_simple() {
    let result = count_source(b"defmodule Main do\n  def run do\n    :ok\n  end\nend\n", "main.ex").unwrap();
    assert_eq!(result.sloc, 5);
}

#[cfg(feature = "elixir")]
#[test]
fn count_elixir_with_comment() {
    let result = count_source(b"# Module comment\ndefmodule Main do\n  def run do\n    :ok\n  end\nend\n", "main.ex").unwrap();
    assert_eq!(result.sloc, 5);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "make")]
#[test]
fn count_makefile_simple() {
    let result = count_source(b"all:\n\techo hello\n", "Makefile").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "make")]
#[test]
fn count_makefile_with_comment() {
    let result = count_source(b"# Build target\nall:\n\techo hello\n", "Makefile").unwrap();
    assert_eq!(result.sloc, 2);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "fsharp")]
#[test]
fn count_fsharp_simple() {
    let result = count_source(b"printfn \"Hello, world!\"\n", "main.fs").unwrap();
    assert_eq!(result.sloc, 1);
}

#[cfg(feature = "lua")]
#[test]
fn count_lua_simple() {
    let result = count_source(b"print(\"hi\")\n", "script.lua").unwrap();
    assert_eq!(result.sloc, 1);
}

#[cfg(feature = "lua")]
#[test]
fn count_lua_with_comment() {
    let result = count_source(b"-- comment\nprint(\"hi\")\n", "script.lua").unwrap();
    assert_eq!(result.sloc, 1);
    assert_eq!(result.comments, 1);
}

#[cfg(feature = "php")]
#[test]
fn count_php_simple() {
    let result = count_source(b"<?php\necho \"hi\";\n", "index.php").unwrap();
    assert_eq!(result.sloc, 2);
}

#[cfg(feature = "dart")]
#[test]
fn count_dart_simple() {
    let result = count_source(b"main() {\n  print('hi');\n}\n", "main.dart").unwrap();
    assert_eq!(result.sloc, 3);
}

#[cfg(feature = "swift")]
#[test]
fn count_swift_simple() {
    let result = count_source(b"print(\"hi\")\n", "main.swift").unwrap();
    assert_eq!(result.sloc, 1);
}

#[cfg(feature = "solidity")]
#[test]
fn count_solidity_simple() {
    let result = count_source(b"pragma solidity ^0.8.0;\ncontract Main {}\n", "Main.sol").unwrap();
    assert_eq!(result.sloc, 2);
}
