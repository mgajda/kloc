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
kloc --exclude json,yaml # All machine language files
                         # except json and yaml
kloc --only-programming # count only programming languages
                        # (not configuration and markup languages)
```

## Packaging status

Debian, RPM, Podman, and Snap packages are not yet available — contributions welcome.
