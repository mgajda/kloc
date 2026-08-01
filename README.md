# kloc — count lines of code and code complexity via universal AST parsing

New approach to counting lines of code and code complexity based on a
universal AST parsing engine — tree-sitter.  Makes the code lean and easy
to maintain by depending on the diligent work of the tree-sitter community.

`sloccount` (David A. Wheeler) and `cloc` (Al Danial) are Debian/Ubuntu
packages with a similar purpose, but not using universal parsers.
I found sloccount invaluable in understanding the pace of my own work
and the size of other projects in the past. However I was disappointed
that it received no maintenance.
`cloc` on the other hand seems slow, and uses regular expressions,
which make me trust results less than AST parser used daily by developers.

## Build

Requires Rust 1.85+.

```sh
git clone https://github.com/mgajda/kloc.git
cd kloc
cargo build --release
```

## Install

```sh
cargo install --path .
```

Binary is installed as `~/.cargo/bin/kloc`.

kloc's greater precision comes at a price — the binary exceeds 100 MB
when built with all supported languages, because each language bundles
a tree-sitter parser (a C library compiled into the binary).  Default
features include only programming languages; enable `all-languages`
for the full set.

## Use

```sh
kloc                    # count current directory
kloc src/               # count specific directory
kloc file1.rs file2.c   # count specific files
kloc --json             # JSON output
kloc --only rust        # only Rust files
kloc --exclude json,yaml
kloc --only-programming
kloc --full             # show detailed Halstead/McCabe metrics
kloc --sloc-only        # skip complexity analysis (faster)
kloc --no-cache         # disable the on-disk result cache
kloc --color always     # force colors (auto/always/never; default auto)
kloc --ignore build     # also ignore a directory named 'build'
kloc --no-ignore node_modules  # stop ignoring node_modules (a default)
kloc --no-ignore-defaults     # ignore nothing by default
kloc --history          # analyze git history (changed tokens, AI time to process)
kloc --history --from v1.0          # range: from v1.0.. (to branch tip)
kloc --history --from v1.0 --to v2.0  # range: v1.0..v2.0
kloc --ai-config /path/ai.toml   # use a custom AI-platform config
kloc --write-ai-config           # write the embedded default AI config and exit
kloc --ai-multiplier 10          # override the AI effort multiplier for all platforms
```

By default the walker skips dependency / build-cache directories:
`node_modules`, `.git`, `.vscode`, `.opencode`, `.claude`, `.cache`,
`__pycache__`, `dist`, `dist-newstyle`, `.stack`, `.cabal`, `target`.
Add or remove patterns with `--ignore` / `--no-ignore`, or disable the whole
set with `--no-ignore-defaults`. Patterns match a directory name (not a path),
so `--ignore dist` skips any `dist/` at any depth.

The schedule/effort block is a grouped table: rows are Metric / Effort / Team
size / Schedule; columns are the estimation methodologies, grouped into
families (LoC-driven, AST-driven, AI), each column a distinct colour. The
**AI** columns are LLM-based: their metric and effort are token counts (with an
ISO magnitude suffix, e.g. `595k tokens`), and their schedule is the
platform's plan-cap time to process those tokens, counted by plan caps
(5h window / day / week / month) rather than a linear rate (no team size).
The **Halstead** column shows an *optimal team size* derived from the COCOMO II
schedule relationship, so its schedule is the parallelized time (not the
single-developer `E/18s` figure, which is shown separately as "Time to
implement"). Duration units run up to years, kya (thousands of years), and
Mya (millions of years).

Output is concise by default: per-language SLOC, total code/comment lines,
token counts, Halstead time-to-implement, average cyclomatic complexity,
a schedule table (rows = Schedule/Effort/Team size, columns = methodologies),
and a performance summary (GB/s, files/s, declarations/s, total runtime).

When stdout is a terminal, each language is shown in its GitHub logo colour
(from `ozh/github-colors`; unique in both hex and the 256-colour palette,
with a darker derived background for two-tone logos), the schedule-table
columns are colour-coded, and the performance section is dimmed grey.
Colour is auto-detected but can be forced or disabled with
`--color always|never` (honours `NO_COLOR`).

Token counts are always computed:
- **Tree-sitter tokens** — leaf tokens and named nodes of the concrete
  syntax tree (no extra dependency).
- **LLM tokens** — counted with the gigatoken tokenizer (a fork that builds
  on stable Rust) using two embedded tokenizer specifications:
  - **DeepSeek V4** — the `deepseek-ai/DeepSeek-V4-Flash` byte-level BPE
    tokenizer (MIT).
  - **Claude Sonnet** — the official Anthropic `claude.json` (same file as
    in `@anthropic-ai/tokenizer`, MIT), matching the reference `countTokens`
    including its NFKC normalization.
  These lines only appear when the binary was built with
  `cargo install --path . --features tokens`.

## History mode

`kloc --history` streams `git log -p` and counts the tokens changed
(added + modified + removed) across the repository's commit history, per
language, then estimates how long it would take to process those tokens on
each configured AI platform. Build with `--features tokens` so the
changed-token count is real; without it the token counts are zero.

## AI-platform config

AI platforms are defined in a TOML config file (multiple platforms, each with
its own caps and effort multiplier), so providers can be calibrated
independently. The default config is **embedded in the binary** and is used
when no file is found. Discovery order: an explicit `--ai-config <path>`, else
`$XDG_CONFIG_HOME/kloc/ai.toml` (or `~/.config/kloc/ai.toml`). `--write-ai-config`
writes the embedded default to disk so you can edit it; `--ai-multiplier`
overrides every platform's effort multiplier at once.

Each platform entry has `name`, a monotonic `caps` list of `(tokens,
duration_seconds)` breakpoints, and an optional `multiplier` (AI effort:
effective tokens = tokens × (1 + multiplier); 3–5x standard, 10–20x complex
reasoning). The figures are **approximate calibration**: Anthropic's help
centre publishes only the relative plan multiples (Max 5x = 5× Pro, Max 20x =
20× Pro), not absolute token numbers; the Pro baseline (~44k tokens per
5-hour window) follows the widely reported figure (faros.ai, Dec 2025; Claude
Code 5-hour limits were doubled in May 2026). DeepSeek V4 is estimated from
OpenCode Go usage limits.

## Testing

`cargo test --all-features`. The git-history consistency tests generate
random tree-sitter-parseable Rust code: each run uses a fresh seed (reported
as `KLOK_TEST_SEED=…` in test output) and honours the `KLOK_TEST_SEED` env
var to replay a specific seed deterministically. See `AGENTS.md`.

## Packaging status

Debian, RPM, Podman, and Snap packages are not yet available — contributions welcome.
