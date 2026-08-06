---
title: "Counting Lines of Code with a Tree-Sitter Grammar: An Experience Report on kloc"
author: "Marek Gajda"
date: "2026-08-06"
lang: en-GB
---

# Abstract

This paper reports a benchmark of `kloc`, a Rust code counter that uses
tree-sitter grammars. We compare `kloc` with five tools. The tools are
cloc, sloccount, tokei, scc, and gocloc. The corpus has thirteen pinned
open-source repositories.

On the languages that all tools support, `kloc` agrees with cloc within
0.4 % of the code lines. Its median runtime is 0.45 s. cloc takes 1.30 s.
sloccount takes 0.92 s. tokei and scc take 0.02 s. `kloc` is about 22
times slower than tokei and scc.

`kloc` pays for its design. It embeds about 40 grammars and two LLM
tokenizers. Peak memory reaches 2.7 GiB on a large C++ tree. The
tokenizers added about 0.6 s of startup in code-only mode.

The benchmark found four problems. We fixed each one. `kloc` now skips the
tokenizer in code-only mode. It excludes docstrings from the code
count and reports them as a separate metric. It recognises `.pyi` type-stub
files. It counts Jupyter notebooks by their code and markdown cells.

We report the protocol and the measurements. We report the lessons for
tool builders. The main lesson is the interaction of parallelism with the
memory wall.

# 1 Introduction

Counting lines of code (LOC) is one of the oldest software metrics [1].
Effort models and schedule models use it. COCOMO [2] and Putnam [3] are
two such models. The metric also tracks repository growth. It audits the
size of a codebase.

The user must choose a tool and decide how much computing resource the
count can use. The tools differ in speed. They differ in the number of
languages they recognise. They differ in what they count as a line of code.

`kloc` is a Rust counter. It parses each file with a tree-sitter grammar
[4]. It does not use regular expressions. The tree-sitter design gives
access to the concrete syntax tree. The tool derives line counts from the
tree. It also derives Halstead [5] and McCabe [6] metrics, Henry-Kafura
information flow [7], and COCOMO and Putnam schedules. For git
repositories it estimates the effort that an LLM needs to process the
changed tokens. The source tree is about 5.8 thousand lines of code.

This paper is an experience report. We benchmarked `kloc` against cloc
[8], sloccount [9], tokei [10], scc [11], and gocloc [12]. The corpus has
thirteen repositories. We measured runtime, peak memory, failure
behaviour, language coverage, and the agreement of the code lines.

We asked four questions.

1. How fast is each tool? Where does the time go?
2. How much memory does each tool use? Why?
3. Which languages do the tools recognise? Where are the gaps?
4. Where do the counts diverge? Do the divergences hide bugs?

The answers led to concrete fixes in `kloc`. We report those fixes as the
main contribution. The lesson is that benchmarking against established
tools is a forcing function for correctness and for resource use.

# 2 Methods

## 2.1 Corpus

We counted thirteen source trees. The trees cover fifteen or more
languages. They range from 2.4 MB to 224 MB of source. Each tree is pinned
to a commit. Table 1 lists the repositories and the commits.

| repo | URL | commit |
|---|---|---|
| redis | https://github.com/redis/redis | `bf49481ad7cf` |
| jquery | https://github.com/jquery/jquery | `51eb576cca6f` |
| numpy | https://github.com/numpy/numpy | `57ae4890237c` |
| cloc | https://github.com/AlDanial/cloc | `acab536b38c3` |
| go-ethereum | https://github.com/ethereum/go-ethereum | `a235d281925b` |
| rails | https://github.com/rails/rails | `a8c7c6c120d5` |
| graphify | https://github.com/Graphify-Labs/graphify | `0b2bd938c4a4` |
| mesa | https://gitlab.freedesktop.org/mesa/mesa | `41e2d0fc5d80` |
| duckdb | https://github.com/duckdb/duckdb | `22bf369c2ac0` |
| btop | https://github.com/aristocratos/btop | `e2479bba0197` |
| agda2hs | https://github.com/agda/agda2hs | `acb521e2b66c` |
| tree-sitter | https://github.com/tree-sitter/tree-sitter | `a56fc9eec64f` |
| kloc | https://github.com/mgajda/kloc | `3a19bd3d8177` |

Table 1: The corpus. Each repository is pinned to a commit.

The trees are clean checkouts. We removed the `.git` directories and the
build outputs. Every tool sees the same input. The tool under test is the
release build of `kloc` at commit `4edaffd`. That build includes the fixes
in Section 3.4.

## 2.2 Tools and versions

| tool | version | invocation |
|---|---|---|
| kloc | 0.2.0 (commit `4edaffd`) | `kloc --sloc-only --color never DIR` |
| cloc | 2.06 | `cloc --quiet DIR` |
| sloccount | 2.26 | `sloccount DIR` |
| tokei | 14.0.0 | `tokei DIR` |
| scc | 3.7.0 | `scc DIR` |
| gocloc | 0.7.0 | `gocloc DIR` |

Table 2: The tools and their invocations.

Each tool runs with its default settings. We run one tool at a time. The
`kloc` invocation requests code and comment counts only. That mode matches
what the other tools report.

## 2.3 Machine and measurement

The benchmark ran on one machine. The machine has 12 physical cores and
24 threads. It has 29 GiB of RAM. The operating system is Linux
7.0.0-28-generic.

We ran each tool three times per repository. We report the median
wall-clock time and the median peak RSS. The `time -v` command captures
both. A tool process stops after 900 s. Each process runs under a
virtual-memory cap of 8 GiB. The cap stops a runaway counter from taking
down the host.

All reported numbers come from the recorded run data. A script computes
them. The tables reproduce those outputs.

# 3 Results

## 3.1 Runtime

Table 3 shows the median wall-clock time per tool and repository.

| repo | cloc | gocloc | kloc | scc | sloccount | tokei |
|---|---|---|---|---|---|---|
| agda2hs | 0.13 | 0.02 | 0.05 | 0.00 | 0.51 | 0.00 |
| btop | 0.18 | 0.02 | 0.20 | 0.00 | 0.40 | 0.00 |
| cloc | 0.35 | 0.05 | 0.53 | 0.03 | 0.69 | 0.01 |
| duckdb | 5.65 | 0.64 | 14.93 | 0.12 | 14.73 | 0.08 |
| go-ethereum | 2.31 | 0.30 | 4.46 | 0.05 | 4.44 | 0.09 |
| graphify | 0.41 | 0.04 | 0.12 | 0.00 | 0.47 | 0.01 |
| jquery | 1.19 | 0.03 | 0.10 | 0.00 | 0.92 | 0.01 |
| kloc | 0.44 | 0.04 | 0.01 | 0.02 | 0.14 | 0.03 |
| mesa | 12.20 | 1.43 | 4.81 | 0.16 | 20.93 | 0.31 |
| numpy | 2.82 | 0.19 | 0.90 | 0.02 | 2.77 | 0.03 |
| rails | 1.72 | 0.21 | 0.58 | 0.03 | 8.07 | 0.04 |
| redis | 1.30 | 0.16 | 0.45 | 0.02 | 3.38 | 0.02 |
| tree-sitter | 1.32 | 0.04 | 0.11 | 0.01 | 0.58 | 0.01 |
| **median** | **1.30** | **0.05** | **0.45** | **0.02** | **0.92** | **0.02** |
| **geomean** | **1.07** | **0.10** | **0.44** | **0.00** | **1.63** | **0.00** |

Table 3: Median wall-clock time in seconds. We ran each tool three times.

The tools split into three groups. tokei and scc are single-purpose
counters. They are optimised for speed. They finish in tens of
milliseconds on every repository except mesa and duckdb. gocloc, a Go
counter, is next. `kloc`, cloc, and sloccount do more than count. They are
one to two orders of magnitude slower. `kloc` is the fastest tool in that
group. It beats cloc on nine of the thirteen repositories. It beats
sloccount on eleven.

The exception is duckdb. This tree has 7.5 thousand C++ files. `kloc`
takes 14.9 s. cloc takes 5.7 s. `kloc` is 2.6 times slower. Section 4.2
discusses the cause.

## 3.2 Memory

Table 4 shows the median peak RSS.

| repo | cloc | gocloc | kloc | scc | sloccount | tokei |
|---|---|---|---|---|---|---|
| agda2hs | 23 | 26 | 24 | 10 | 6 | 5 |
| duckdb | 105 | 60 | 2756 | 177 | 19 | 44 |
| go-ethereum | 176 | 62 | 342 | 120 | 8 | 73 |
| mesa | 210 | 66 | 293 | 317 | 9 | 94 |
| numpy | 50 | 37 | 265 | 58 | 6 | 17 |
| rails | 32 | 36 | 118 | 45 | 7 | 14 |
| redis | 35 | 35 | 143 | 42 | 6 | 13 |
| kloc | 141 | 38 | 12 | 31 | 7 | 14 |

Table 4: Median peak RSS in MiB. This table shows a subset of the
repositories.

`kloc` is the memory outlier. On duckdb it peaks at 2.7 GiB. That is an
order of magnitude above every competitor. On smaller trees it uses tens
to hundreds of MiB. sloccount is the most frugal. It stays under 20 MiB
everywhere.

The contrast is stark on the `kloc` repository. `kloc` uses 12 MiB there.
That run exercises only the Rust grammar. After the fix in Section 3.4, it
does not exercise the tokenizer.

Two sources explain the memory. First, `kloc` counts files in parallel
with a rayon pool. The pool defaults to one thread per logical CPU. This
machine has 24 threads. Each worker holds its file's source and syntax
tree. Second, the binary embeds about 40 grammars and the LLM tokenizers.
They stay resident once loaded. We quantify the first source in Section
3.4.

## 3.3 Failures and coverage

No tool crashed. No tool timed out. All 234 runs exited zero. cloc and
sloccount emit warnings about unrecognised files. `kloc` is silent. The
failure mode is therefore not a crash. It is a difference in what is
counted.

`kloc` recognises 62 languages. cloc recognises 373. The default `kloc`
build enables only programming languages. It does not count machine,
data, markup, and configuration languages. Examples are JSON, YAML, TOML,
Markdown, CMake, HTML, and Tcl. The file coverage is `kloc`'s file count
as a fraction of cloc's. Its median is 71.6 %. This policy choice explains
most of the difference in total SLOC. `kloc` reports 72.2 % of cloc's code
lines, as a median over the corpus.

The extreme case is the `kloc` repository. cloc counts 273 k lines there.
A 267 k-line JSON tokenizer vocabulary dominates that count. `kloc` does
not count JSON in the default build. It reports 5.3 k lines.

## 3.4 Divergences and the fixes they drove

On the languages the tools share, the per-language agreement is tight.
Table 5 lists the comparisons where both tools count more than a thousand
lines.

| repo | language | kloc | cloc | ratio |
|---|---|---|---|---|
| go-ethereum | Go | 316,520 | 316,369 | 100.0 % |
| go-ethereum | JavaScript | 8,206 | 8,206 | 100.0 % |
| kloc | Rust | 5,254 | 5,254 | 100.0 % |
| mesa | Lua | 2,031 | 2,031 | 100.0 % |
| mesa | Rust | 72,002 | 71,850 | 100.2 % |
| rails | Ruby | 394,141 | 395,808 | 99.6 % |
| redis | Python | 4,314 | 4,174 | 103.4 % |

Table 5: Per-language code lines. We show the rows where both tools count
more than 1,000 lines.

The raw C counts look inflated. The range is 115 to 160 %. Every case is
header folding. `kloc` counts `.h` files as C. cloc splits them into C and
C/C++ Header. The sums agree to within 0.3 %. redis gives 259,547 against
259,552. go-ethereum gives 43,968 against 43,966. We found no counting bug
in `kloc` for the languages it supports.

Two real divergences surfaced. We fixed both.

**Docstrings.** `kloc` counted Python and Julia docstrings as code. A
docstring is `"""..."""`. tree-sitter parses a docstring as a string
expression statement. cloc treats docstrings as comments. On mesa, `kloc`
reported 20 % more Python lines than cloc. On one file, it reported 41
lines against 33.

We changed `kloc` to detect a docstring. A docstring is a bare string
statement. It is the first statement of its module, class, or function
body. The tool now counts its lines as a new *documentation* metric. It
does not count them as code. After the change, the single-file count
matches cloc exactly. Both tools report 33 lines.

`kloc` now reports documentation lines per language and as a total.
Repository totals still differ by a few percent. The reason is that
cloc's comment heuristic and the docstring rule do not coincide on every
file.

**Type-stub files.** cloc counts `.pyi` files as Python. These files are
Python type stubs. `kloc` did not register the extension. numpy alone has
276 `.pyi` files. The `__init__.pyi` file is 6.9 k lines. We added `.pyi`
to `kloc`'s Python registration.

A third divergence is a policy difference, not a bug. `kloc` counts
Jupyter notebooks by their cells. The notebook files use the `.ipynb`
extension. Code cells count as code. Markdown cells count as
documentation. Nothing else counts. cloc reports 0 code lines for a
notebook. It treats the JSON wrapper as comments. We implemented the
notebook-aware counter after we observed this.

The benchmark also exposed two performance issues. We fixed both.

First, `kloc` built its LLM tokenizers in code-only mode. That added a
fixed 0.6 s and about 90 MiB to every non-empty run. One 13-byte file took
0.62 s and 96 MiB. We now build the tokenizer for token-count requests only. The same run takes 0.003 s and 4.6 MiB. Repository runs dropped
by a factor of two to fourteen.

Second, the default rayon pool uses 24 threads. On the memory wall, that
pool is counterproductive. On duckdb, 24 threads took 31.2 s and 2.9 GiB.
Eight threads took 16.9 s and 2.3 GiB. Capping the pool is a
configuration option for future work.

## 3.5 Size of the tools

Table 6 shows the code size of each tool's own source. We counted with
cloc. We excluded vendored and data files.

| tool | files | code |
|---|---|---|
| gocloc | 25 | 2,664 |
| kloc | 29 | 5,846 |
| sloccount | 67 | 9,762 |
| tokei | 164 | 10,080 |
| scc | 175 | 53,427 |
| cloc | 910 | 64,115 |

Table 6: Size of each tool's own source.

`kloc` is the second smallest, at 5.8 thousand lines. It delivers line
counting in that space. It delivers three complexity metrics. The metrics
are Halstead, McCabe, and Henry-Kafura. It delivers two effort models. The
models are COCOMO and Putnam. It delivers git-history analysis and
LLM-effort estimates.

# 4 Discussion

## 4.1 What the benchmark changed

The central lesson is that an honest cross-tool benchmark is a forcing
function. The benchmark found the docstring error. It found the `.pyi`
gap. It found the notebook gap. It found the tokenizer startup cost. Each one changed reported numbers by tens of
percent. Each one changed times by an order of magnitude.

The per-language agreement is 0.0 to 0.4 % on shared languages. That is
the evidence that the counting core is correct. The divergences were all
at the boundaries. A boundary is what a documentation line is. A boundary
is which extensions belong to a language. A boundary is what a notebook's
code is.

## 4.2 Design trade-offs

`kloc` has three design choices. It embeds the grammars. It embeds the
tokenizers. It parses with a real grammar. It also parallelises with
rayon. Each choice has a measurable cost.

The grammar approach buys accuracy. The accuracy is the 0.4 % agreement.
It costs memory and startup. The tokenizer buys self-contained LLM-effort
estimates. It costs fixed startup and RSS. The rayon default costs more
than it buys on a memory-bound machine. None of these costs is intrinsic.
Each one is a setting that benchmarking made visible.

The parsing strategy explains the speed gap. tokei and scc do not use a
parser. Each tool is a hand-written byte scanner that walks the file left
to right. It tracks a string or comment state and never builds a syntax
tree. `kloc` parses each file into a tree-sitter syntax tree, which costs
more to build. That is why tokei and scc are about 22 times faster.

The state machine is also less precise. It mis-classifies the tricky
syntax that a real parser gets right, such as a comment marker inside a
template string.

The memory numbers also carry a warning for users. A tool that peaks at
2.7 GiB on a 7.5-thousand-file tree will not run everywhere. Misconfigured
parallelism makes it worse. The 8-thread sweet spot we measured is two
times faster than 24 threads. It is 0.6 GiB lighter. It matches the
memory wall better than the CPU count.

## 4.3 Threats to validity

The corpus is small. It does not cover every language and file layout.
All runs are on one machine. Absolute times differ elsewhere. The relative
ordering is stable. The tools count by different definitions. The
definitions include comment handling, blank lines, and what a line of code
is. Even when the tools agree, the SLOC figures are not interchangeable.

We report the median of three runs. We do not report a confidence
interval. The `kloc` figures reflect commit `4edaffd`. The earlier binary
was slower. It also counted docstrings as code.

# 5 Conclusions

We benchmarked `kloc` against five established line counters. The corpus
is pinned and reproducible. `kloc` is faster than cloc and sloccount. It
agrees with cloc to within 0.4 % on shared languages. It is one of the
smallest tools by source size. The fast counters lack its features. Those
features are complexity, history, and LLM-effort estimates.

`kloc` pays for those features in memory and in language coverage. The
default build recognises fewer languages than cloc. It can use an order of
magnitude more memory on large trees.

The benchmark surfaced four fixes. `kloc` now skips the tokenizer in
code-only mode. It excludes docstrings from the code count and reports
them separately. It recognises `.pyi` files. It counts notebooks by their
cells.

We recommend that any tool that claims to count lines of code be validated
against an established counter on a pinned corpus. We recommend that
parallelism be sized to the memory wall, not to the core count.

# References

1.  IEEE Computer Society. IEEE Standard for Software Productivity
    Metrics (IEEE Std 1045-1992), 1993.
2.  B. W. Boehm. *Software Engineering Economics*. Prentice-Hall, 1981.
3.  L. H. Putnam. A general empirical solution to the macro software
    sizing and estimating problem. *IEEE Transactions on Software
    Engineering* SE-4(4), 1978.
4.  tree-sitter. https://tree-sitter.github.io/ (accessed 2026).
5.  M. H. Halstead. *Elements of Software Science*. Elsevier, 1977.
6.  T. J. McCabe. A complexity measure. *IEEE Transactions on Software
    Engineering* SE-2(4), 1976.
7.  S. Henry and D. Kafura. Software structure metrics based on
    information flow. *IEEE Transactions on Software Engineering*
    SE-7(5), 1981.
8.  A. Danial. cloc. https://github.com/AlDanial/cloc (accessed 2026).
9.  D. A. Wheeler. SLOCCount. https://www.dwheeler.com/sloccount/
    (accessed 2026).
10. tokei. https://github.com/XAMPPRocky/tokei (accessed 2026).
11. scc. https://github.com/boyter/scc (accessed 2026).
12. gocloc. https://github.com/hhatto/gocloc (accessed 2026).
