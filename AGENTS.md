# kloc development notes

## Concurrency cap: 4 simultaneous threads (recorded)

This machine has 12 physical cores, but the memory wall hits much earlier. Cap
all parallelism at **4** simultaneous threads — for Cargo (`CARGO_BUILD_JOBS=4`),
for any other build/test job, and for benchmarking runs (only one tool at a
time). A job that needs more cores for throughput must be justified and approved
first.

**cargo-mutants is capped at 4 jobs, never more** — each mutant triggers a full
`cargo test --no-run` rebuild plus a test run, so even 4 workers swamp the
machine. Two knobs multiply together, so BOTH must stay low: the worker count
(`-j`, CLI-only) and the nested cargo build jobs. The record launch is
`-j 2` workers + `.cargo/mutants.toml` `additional_cargo_args = ["-j", "2"]`
(2 workers × 2 nested jobs = 4 concurrent max). Never pass `-j > 4` on the
command line and never raise the nested `-j` above 2.
Launch inside a memory-bounded scope (`systemd-run --user --scope -p MemoryMax=10G
-p MemorySwapMax=0`, see `tmp/launch-mutants.sh`) and run with **one worker**
(`-j 1`): mutant `delete !` in the tree-traversal loops turns them into
unbounded loops that grow memory until the machine OOM-kills the run. The
escape is `ulimit -v 8000000` inside the scope: the hung Vec growth hits
RLIMIT_AS, returns ENOMEM, and Rust's allocator ABORTS (SIGABRT) — a normal
process death that never invokes the kernel OOM killer, so systemd does NOT
mark the scope failed (it stops a unit on ANY kernel OOM-kill, even a correctly
isolated one; a low MemoryHigh soft-limit instead triggers systemd-oomd's
pressure-kill). The cgroup MemoryMax is only a high safety net. Do NOT raise
the worker count above 1 and do NOT raise `--timeout` above 60: two hang tests
at once or a long hang both defeat the isolation.

## Randomized tests: seed SOP (recorded)

Property / random tests (e.g. history-consistency vs source-tree consistency)
MUST use real randomness, not a fixed deterministic seed:

- **Different seed every run** — derive the seed from system entropy at
  startup (e.g. `SystemTime` / `OsRng`), not a hard-coded constant.
- **Report the chosen seed** — print it to the test output (`println!`), so a
  failure can be reproduced and filed.
- **Explicit seed override** — honour an environment variable (e.g.
  `KLOK_TEST_SEED`) so a reported failing seed can be replayed deterministically.
- **On failure, capture the seed + failing input** as a fixed regression test.

A randomized test with a fixed seed is not a randomized test — it is a
deterministic test wearing a disguise, and it misses the diversity that
property testing is meant to find.

This convention is mirrored from the global
`~/.config/opencode/AGENTS.md` testing rules (which live outside this repo and
cannot be edited from here).

## Container / podman builds are not local tests

Never build the container image or run podman as part of the local test
flow — `cargo test` and any local verify step must skip them. These builds
compile ~40 grammars plus gigatoken with fat LTO (~15-25 min, ~8 GB RAM; see
"Building packages" in README.md) and are only:
- CI: `.github/workflows/package.yml` (runs on `v*` tags or manual dispatch),
- local: the opt-in `./build-container.sh [--measure]` script.
