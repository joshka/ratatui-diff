# Syntax preparation and footprint

[Download the raw measurements and frozen executables](https://github.com/joshka/ratatui-diff/releases/download/review-media-2026-10-09/syntax-performance-evidence-20261009.tar.gz).

The optional `syntax` feature prepares owned styles outside rendering. Complete snapshots preserve
lexical state from omitted prefixes; retained input trades that accuracy for less parsing. The
[syntax contract](syntax-styling.md) defines source validation, limits, and style composition.

## Measurement conditions

Measured October 9, 2026 using library revision `b5d6930c` with the benchmark additions in this
change, on an Apple M2 Max with 96 GiB RAM, macOS 26.6.2 (25G83), and Rust 1.99.0
(`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, LLVM 23.1.1). Cargo's normal optimized bench/release
profiles are used, with no custom Rust flags, LTO, or profile overrides. Other coordinated tasks
pause builds and measurements during each timing window. Background OS work remains uncontrolled.

The [measurement manifest](syntax-performance-manifest.json) records the full base revision,
measured benchmark source hashes, compiler identity, executable hashes, effective sample count,
commands, and the exclusive window (136 seconds). The report's additions leave library code intact.

The locked graph uses Syntect 5.3.0, two-face 0.5.2+bat-0.26.1, Oniguruma `onig` 6.5.3 / `onig_sys`
69.9.3, and bincode 1.3.3. These versions describe the evidence, not new minimum requirements.

Criterion runs use 10 samples, 100 ms warm-up, and 200 ms requested measurement. Expensive
operations extend collection beyond that request. Point estimates and confidence intervals are
retained in raw JSON. Small differences from a single short run are not established regressions;
these measurements are not absolute CI gates.

## Preparation results

Values are Criterion sample-mean point estimates. Brackets show the bootstrap 95% confidence
interval, rounded; each dense workload inserts all source lines.

| Workload           | Full input          | Retained input       |
| ------------------ | ------------------- | -------------------- |
| Rust, 1,000 lines  | 44.3 ms [43.9–44.8] | 44.4 ms [44.0–44.9]  |
| Multiline, 1,000   | 9.70 ms [9.62–9.77] | 9.86 ms [9.69–10.06] |
| Unicode, 1,000     | 26.8 ms [26.6–27.1] | 26.5 ms [26.2–26.9]  |
| Long lines, 1,000  | 676 ms [668–684]    | 698 ms [671–746]     |
| Sparse EOF, 10,000 | 199 ms [196–202]    | 130 µs [128–132]     |

The original Unicode Rust fixture takes 26.0 ms [25.6–26.5] for 1,000 lines and 2.72 s [2.61–2.87]
for 100,000 lines using full input. Retained input does comparable work when all source is retained.
The sparse EOF result reflects parsing omitted prefixes on both sides. Its retained result lacks the
opening block-comment state and is not an equivalent accuracy guarantee.

Warmed highlighter construction takes 5.81 µs [5.70–5.98]. Construction plus contrast adaptation to
`aardvark_ink` takes 39.5 µs [36.2–43.4]. These loops include theme cloning and destruction; they
exclude cold shared assets and the first theme decode.

Five fresh 100,000-line probes give the following medians and observed ranges. Asset initialization
loads the shared syntax/theme containers, not every grammar's lazy parser state.

| Fresh-process phase                   | Median  | Observed range |
| ------------------------------------- | ------- | -------------- |
| Shared assets through `themes()`      | 1.06 ms | 0.961–1.21 ms  |
| First `Coldark-Dark` decode and clone | 114 µs  | 98–136 µs      |
| Cached theme construction             | 6 µs    | 6–19 µs        |
| First Unicode preparation, 100,000    | 2.82 s  | 2.66–2.94 s    |

The first preparation includes lazy Rust grammar work. At 1,000 lines its fresh-process median is
28.8 ms (28.1–29.3 ms). The probe's cached construction retains the instance until after its timer,
so it excludes destruction and is not the same timing boundary as Criterion construction.

## Layout and steady rendering

First-layout sample means use the original Unicode Rust fixture and prepared styles held in memory
for both columns. Syntax attachment adds visible painting without changing the geometry contract.
Intervals overlap in several pairs; this run does not establish a layout regression.

| Wrapped mode | Lines   | Unattached layout   | Attached layout      |
| ------------ | ------- | ------------------- | -------------------- |
| Unified      | 1,000   | 1.31 ms [1.29–1.33] | 1.35 ms [1.32–1.39]  |
| Split        | 1,000   | 1.34 ms [1.31–1.36] | 1.50 ms [1.35–1.74]  |
| Unified      | 100,000 | 146 ms [145–147]    | 153 ms [147–163]     |
| Split        | 100,000 | 152 ms [151–154]    | 157 ms [152–165]     |

The initial short split/1,000-line scroll samples were noisy: the unattached mean was 126 µs
[105–154] and the attached mean was 175 µs [150–203]. All eight scroll cases were repeated with 300
ms warm-up and one second of measurement to assess that variance. The group fixes ten samples; the
repeat command requested twenty, but the effective count remained ten. Repeat sample means:

| Wrapped mode | Lines   | Unattached frame    | Attached frame      |
| ------------ | ------- | ------------------- | ------------------- |
| Unified      | 1,000   | 91.3 µs [87.1–96.9] | 95.6 µs [95.2–96.0] |
| Split        | 1,000   | 90.2 µs [89.8–90.5] | 99.0 µs [98.6–99.3] |
| Unified      | 100,000 | 92.2 µs [91.7–92.6] | 104 µs [102–106]    |
| Split        | 100,000 | 94.0 µs [93.5–94.6] | 103 µs [102–103]    |

Styled frame work grows much less than the 100-fold source increase in these fixtures. A frame still
visits the viewport and looks up prepared ranges; these results do not bound arbitrary long lines,
larger viewports, other grammars, unwrapped horizontal scrolling, or terminal backends.

## Binary and resident footprint

The release examples have no extra dynamic-library dependency in `otool -L`. The syntax example
links the Oniguruma implementation and exposes the embedded license-printing path.

| Release example | Unstripped bytes | Stripped bytes |
| --------------- | ---------------- | -------------- |
| Default         | 1,592,832        | 1,311,936      |
| Syntax feature  | 3,580,448        | 3,142,912      |
| Increase        | 1,987,616        | 1,830,976      |

The two-face input assets selected by this build total 1,057,992 bytes: newline-aware Oniguruma
grammars 983,684, lazy themes 62,593, and compressed acknowledgements 11,715. These lengths are
embedded input files, not decoded heap sizes. Executable growth also includes parser, decoder,
native backend, and adapter code plus linker layout differences. Feature-enabled applications that
never reference syntax entry points may link a different amount.

Matched example peak RSS medians are 2.31 MiB default (2.31–2.33), 2.42 MiB with the feature enabled
but unattached (2.41–2.44), and 7.09 MiB after preparing/attaching syntax (7.02–7.13). All print the
same row count. This small complete-source example is not the 100,000-line probe.

Current probe RSS medians, in KiB, include a different executable and larger generated inputs:

| Phase                         | 1,000 lines | 100,000 lines |
| ----------------------------- | ----------- | ------------- |
| Before asset loading          | 2,000       | 2,000         |
| Shared assets                 | 5,280       | 5,280         |
| First selected theme          | 5,504       | 5,472         |
| Document and source input     | 6,304       | 36,208        |
| Prepared styles               | 7,936       | 67,840        |
| Input and highlighter dropped | 7,936       | 67,856        |
| First split/wrapped layout    | 12,560      | 499,248       |
| After 1,000 styled frames     | 12,576      | 499,312       |
| Styles dropped                | 12,576      | 499,328       |

The 100,000-line preparation raises current RSS by roughly 31 MiB before layout. Layout then adds
roughly 421 MiB in this fixture. Neither delta is an exact object allocation size: shared lazy
grammar state, scratch allocation, allocator retention, and resident pages participate. Style
destruction does not reduce RSS in these samples. The shared asset containers live for the process;
these observations cannot establish their exact decoded allocation footprint or reclamation cost.

## Workloads and timing boundaries

`benches/support/syntax.rs` owns generated deterministic fixtures. Source comparison, document
construction, shared asset initialization, and initial grammar compilation occur outside warmed
Criterion preparation timing. Returning and destroying the owned style output remain timed.

- The original preparation/render fixture inserts 1,000 or 100,000 numbered Rust declarations
  containing `界 café`, a string, and a line comment into an empty source.
- The Rust workload inserts 1,000 functions with a numbered integer, local binding, and expression.
- The multiline workload inserts 1,000 lines cycling through an opening block comment, nested block
  comment body, closing comment, and raw string declaration.
- The Unicode workload inserts 1,000 declarations with wide characters, a combining accent, and a
  ZWJ emoji inside a string, plus an accented line comment.
- The long-line workload inserts 1,000 declarations whose string repeats `界 café` followed by a
  space 400 times. Each source line remains below the default 64 KiB line limit.
- Sparse EOF input appends one declaration to 10,000 multiline lines. The document retains three
  context lines and the insertion. Full input parses both prefixes; retained input starts within the
  final comment and cannot reconstruct its omitted opening delimiter.

Full and retained preparation use the same document and highlighter for each pair. Dense workloads
retain all source, while sparse input exposes prefix parsing. Both sides parse independently; full
preparation stops after each side's final retained line and still checks the complete source size.

Layout timing starts with a new `DiffState`; it includes first rendering and excludes destruction of
the resulting state through Criterion's batched loop. Steady scrolling reuses layout, advances one
row, and wraps to the start near EOF. A 100×30 buffer, wrapped unified/split modes, and the
identical prepared document are used for attached/unattached pairs. Buffer allocation, syntax
preparation, source comparison, terminal I/O, and host event handling are excluded. Unattached runs
retain the prepared styles in memory.

## Fresh processes and size boundaries

The opt-in bench probe measures shared asset initialization through `themes()`, then first selection
of `Coldark-Dark`, then construction with the cached theme. It samples current RSS using `ps`
outside timed regions. First preparation includes lazy Rust grammar loading/regex compilation. Five
fresh processes per size separate shared assets from document/input, preparation, layout, 1,000
styled frames, and style destruction.

RSS changes include resident executable pages, allocator caching, decoder scratch, parser caches,
and retained style output. Dropping styles need not return pages to the OS. The public API exposes
no allocation census, so these samples cannot isolate exact style bytes or peak decoder memory.

The default and syntax-enabled viewer examples are built from the same source and lockfile with
identical release settings in separate targets. Both run `--fixture syntax --measure wrapped 100`;
the enabled case additionally passes `--syntax Coldark-Dark`. Five fresh invocations use macOS
`/usr/bin/time -l`, whose maximum resident set size is bytes. These peak samples are distinct from
the probe's current RSS in KiB. Unstripped and stripped executable sizes include code and linked
assets; embedded asset file lengths are recorded separately and do not predict total binary growth.

## Reproduction

Use a dedicated target directory to avoid competing Cargo builds. Build before reserving a quiet
measurement window:

```sh
cargo bench --bench viewer --features syntax --locked --no-run \
  --target-dir /tmp/ratatui-diff-syntax-performance
cargo build --release --example viewer --locked \
  --target-dir /tmp/ratatui-diff-syntax-performance/default
cargo build --release --example viewer --features syntax --locked \
  --target-dir /tmp/ratatui-diff-syntax-performance
```

Set `CRITERION_HOME` to an evidence directory outside tracked files. Run the executable printed by
Cargo directly to avoid measuring a rebuild:

```sh
CRITERION_HOME=/absolute/evidence/criterion /absolute/bench-executable \
  'syntax/' --bench --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10 --noplot
CRITERION_HOME=/absolute/evidence/scroll-repeat /absolute/bench-executable \
  'syntax/scroll-' --bench --warm-up-time 0.3 --measurement-time 1 --sample-size 10 --noplot
RATATUI_DIFF_SYNTAX_PROBE=1 RATATUI_DIFF_SYNTAX_LINES=100000 /absolute/bench-executable
/usr/bin/time -l /absolute/viewer-default --fixture syntax --measure wrapped 100
/usr/bin/time -l /absolute/viewer-syntax --fixture syntax --measure wrapped 100
/usr/bin/time -l /absolute/viewer-syntax --fixture syntax --measure wrapped 100 \
  --syntax Coldark-Dark
```

Preserve the raw logs, Criterion sample/estimate JSON, source and executable SHA-256 hashes, build
logs, and host/toolchain manifest together. Generated evidence and executables remain outside
repository history.

This run's raw bundle is retained under `media/syntax-performance-2026-10-09/` in the coordinating
checkout, outside the temporary measurement workspace. It contains the runner, logs, all Criterion
sample/estimate JSON, manifests, the frozen measured bench and unstripped examples, and stripped
example copies. Preserve the bundle separately when moving or deleting local build directories.

## Integration cost and coverage

Oniguruma requires a native C toolchain. This run builds that backend on macOS arm64; clean build
time is not measured. The library MSRV remains Rust 1.98. CI tests all features on stable Linux,
macOS, and Windows, plus Rust 1.98 on Linux. Stable/beta Clippy, docs, and the syntax terminal tape
provide additional hosted coverage. These checks do not measure cross-platform performance or
establish a backend compilation-time budget. No visible rendering or runtime API changes are made.

Local validation passes all 30 syntax cases in Criterion's untimed `--test` mode, the focused bench
Clippy check with warnings denied, nightly formatting, and report Markdown lint. Hosted CI for this
change remains separate; the existing Rust 1.98 syntax graph is unchanged by benchmark additions.
