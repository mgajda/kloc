//! End-to-end CLI tests that spawn the real `kloc` binary. They exercise
//! `main()` (arg parsing, --write-ai-config, --ai-config errors, the run and
//! history paths), which unit tests cannot reach.
//!
//! Spawning the debug binary pays the ~2.5 s debug tokenizer build for the
//! tests that reach `run()`/`run_history()`; those are `#[ignore]`d so the
//! fast default suite skips them. Run them with
//! `cargo test -- --include-ignored` (the coverage pass does).

use std::process::Command;

fn kloc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kloc"))
}

#[test]
fn cli_version() {
    let out = kloc().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("kloc"));
}

#[test]
fn cli_help() {
    let out = kloc().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage"));
}

#[test]
fn cli_write_ai_config_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/ai.toml");
    let out = kloc().arg("--write-ai-config").arg(&path).output().unwrap();
    assert!(out.status.success());
    assert!(path.exists(), "config must be written to {path:?}");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("platforms"), "written config must parse");
}

#[test]
fn cli_invalid_ai_config_file_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "[[[ not toml").unwrap();
    let out = kloc().arg("--ai-config").arg(&path).output().unwrap();
    assert!(!out.status.success(), "invalid AI config must fail");
}

#[test]
#[ignore] // spawns the debug binary, which pays the ~2.5 s tokenizer build
fn cli_sloc_only_runs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), b"fn main() {}\n").unwrap();
    let out = kloc()
        .args(["--sloc-only", "--no-cache"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Total files"));
}

#[test]
#[ignore] // spawns the debug binary, which pays the ~2.5 s tokenizer build
fn cli_json_output_is_parseable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), b"fn main() {}\n").unwrap();
    let out = kloc()
        .args(["--json", "--no-cache"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["total_files"].as_u64().unwrap() >= 1);
}

#[test]
#[ignore] // spawns the debug binary, which pays the ~2.5 s tokenizer build
fn cli_history_runs_on_repo() {
    let repo = env!("CARGO_MANIFEST_DIR");
    let out = kloc()
        .args(["--history", "--from", "HEAD~1"])
        .arg(repo)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Git history"));
}

#[test]
#[ignore] // spawns the debug binary, which pays the ~2.5 s tokenizer build
fn cli_verbose_debug_logging_runs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), b"fn main() {}\n").unwrap();
    let out = kloc()
        .args(["-vv", "--sloc-only", "--no-cache"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
}
