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
```

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

## Packaging status

Debian, RPM, Podman, and Snap packages are not yet available — contributions welcome.
