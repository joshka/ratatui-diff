# Testing

Test observable contracts rather than mirroring implementation. Original fixtures should isolate
metadata, malformed hunks, quoted paths, binary summaries, CRLF, final-newline differences, Unicode,
and control characters. Buffer assertions verify visible content and styles, not terminal setup.

Exercise unified/split alignment, wrapping, wide-cell clipping, navigation boundaries, tiny areas,
resize/mode transitions, replacement, and source mappings. Test caller-supplied highlights and
bounded refinement. Public integration tests should use exported APIs as a host would.

Run focused tests while iterating, then `just check`, `just msrv`, and package validation. CI runs
platform tests and beta Clippy; report pending hosted checks separately from local evidence.

`just bench` separates preparation, first layout, resize, and steady frames. Include 100,000-line
inputs, Unicode, long lines, and dissimilar sources. Record machine, compiler, fixture, and timings;
compare steady viewport work across document sizes. Shared CI does not gate absolute timings.

Betamax tapes exercise the example with output waits and fixed terminal geometry. Generated images
are ignored and belong in release assets, never repository history.

## Terminal Consumer Acceptance

Build this checkout's viewer with `cargo build --example viewer --locked`, then run
`python3 scripts/terminal-ux.py`. The runner resolves the executable through Cargo metadata and
replays the `examples/terminal-ux-*.tape` cases. Set `BETAMAX` to an explicit executable path; set
`BETAMAX_VERSION` to require its reported version. `tool.txt` retains the resolved executable,
version, executable SHA-256, and optional `BETAMAX_REVISION` source identity. Use `--case files` for
the file-folding scenario alone; repeat `--case` to select several cases. Exact Unicode/source
checkpoint checks run when their resize, Unicode, and multi-file tapes are all selected. Every
selected tape still runs its own assertions and retains failure diagnostics.

For integrated acceptance against a frozen executable, pass `--viewer /absolute/path/to/viewer` and
`--viewer-revision COMMIT`; `viewer.txt` retains its path, revision, and executable SHA-256. The
ordinary default still resolves this checkout’s Cargo target.

[Betamax 0.1.22](https://github.com/joshka/betamax/releases/tag/betamax-v0.1.22) is the first
published release with mouse input, live resize, and settled assertions. Its tag resolves to
[`5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2`](https://github.com/joshka/betamax/commit/5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2).
CI downloads the exact Linux release asset and verifies its SHA-256 before running the scenarios.
The `terminal-acceptance` job is blocking through the `required` aggregate; its artifact upload uses
`if: always()` to retain PNG/JSON checkpoints, logs, tool identity, and failure diagnostics.

For local release validation, download the matching 0.1.22 platform asset and verify the digest
reported by the release API. Set `BETAMAX` to the extracted executable, `BETAMAX_VERSION=0.1.22`,
and `BETAMAX_REVISION=5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2`. The macOS arm64 asset digest is
`0fe9e07cf963a668725508fd5afa62120245950fded75718898546c4de22f98e`; the Linux x86-64 asset digest is
`497c0bbf6da724e90be49e48ade317d99567e4b03e4e86f03c0dac691ec6358f`. These released artifacts replace
preliminary source-built evidence from revision `9a7a34597ae08f428b8363269571325e87d0984f`, which
reported 0.1.21 while using unreleased commands.

The cases reuse Betamax's consumer scenarios: real mouse selection checks old/new source previews;
gutter, header, and blank split-side clicks cannot select source. Focused cell/style assertions
check word emphasis, current/other search matches, and selection. Presentation toggles and live
resize retain selected source text. Unicode checkpoints compare the preview with an original
fixture, including combining marks, wide cells, ZWJ sequences, and source newlines. Multi-file
search and navigation cover nine matches and viewport clamping. `c` opens the example's text
preview; these checks make no OS clipboard claim.

The resize tape includes a paced, captioned drag/resize/source-preview GIF. It keeps the mouse
button held across a live grid resize, then verifies that the completed selection retains its exact
source preview as terminal dimensions change.

The context fixture adds keyboard and mouse fold expansion, collapse, search reveal, and live resize
in split/wrapped/monochrome mode. Its paced GIF explains the interaction; PNG/JSON checkpoints and
focused fold cells/styles check the rendered state independently of recording readability.

The files fixture adds header clicks, `Enter` for the current file, and `A`/`Z` for all files. It
exercises text, binary, and mode-only headers alongside independent context controls. Public
file-folding integration tests check source extraction, explicit reveal, navigation, geometry
changes, and document replacement without requiring terminal input.

The statistics case checks supplied text totals across file folding, split/wrapped mode, and tiny
viewports, including separate binary and metadata counts. It checks the summary with one widget row,
its absence when the host allocates zero rows, and its restoration after resize. The alignment case
checks paired changed lines after an unpaired annotation, exact wide-cell continuations, and
original old/new source previews after mouse selection. It also retains the selected source through
narrow wrapped resize.

Logs, checkpoint PNG/JSON, tool identity, and Betamax failure diagnostics live under
`media/terminal-ux/`. The runner returns nonzero if any tape or exact-source check fails and
continues independent tapes to retain useful evidence. Coordinates are zero-based terminal cells for
the fixed fixture and geometry, excluding capture padding and captions. Maintain focused assertions
when intentional layout changes move source cells; do not regenerate whole-screen baselines as an
automatic response to failures.

For parallel acceptance work, hand off an immutable revision snapshot with an explicit commit ID.
Integrate that snapshot in a separate workspace while owners continue independent changes on stable
parent revisions. Pause an owner only briefly when needed to capture a consistent snapshot; never
update its workspace during active edits or captures. Keep source-control mutations with one
coordinator and run them sequentially. Ordinary jj status, diff, and log commands can snapshot the
working copy; use `jj --ignore-working-copy` for history inspection during an active owner's work.
Shared jj storage does not make separate workspace directories interchangeable.

## Visual Evidence During Development and Review

For visible changes, show what is being built throughout development. Capture the baseline before
editing, then capture each meaningful result with Betamax and inspect the images. Include labeled
before/after images in Codex commentary as changes occur, in the final handoff, and in the PR body.
A reviewer should be able to identify the improvement from a couple of images and a short caption.
Do not substitute a list of styling changes or passing tests for visual evidence.

Use the same fixture, terminal dimensions, font, theme, and viewport position for each comparison.
Label intentional differences. Choose views that reveal the change: unified/split, narrow/wrapped,
search/selection states, or metadata as appropriate. Validate meaningful rendered states with
inspected PNG checkpoints. Demonstrate interactions with paced, captioned GIFs when clicks,
expansion/collapse, fold-all, scrolling, or other transitions help explain the behavior. Keep
animation readability separate from cell, state, and static-image validation; demonstrate only
implemented interactions. Capture intermediate states when they explain an interaction. Show
rejected experiments when they clarify a meaningful tradeoff.

Keep baseline and iteration images under distinct names so earlier commentary does not silently
change when captures are regenerated. In Codex, embed images using absolute local paths and name the
visible issue and result. In PRs, embed images from durable GitHub release asset URLs or approved
GitHub attachments; local paths and expiring CI artifacts are not reviewer evidence. Label the two
states, give the capture configuration, and keep the images close to the behavior description.
Verify reviewer-accessible assets before publishing the PR. Asset publication requires authorization
when it has not already been granted for the task; prepare and review captures first.

Track tapes and original text fixtures, never generated media. Visual inspection complements buffer,
style, navigation, and Unicode tests. Note unreviewed states and remaining defects honestly. Changes
with no visible effect should say so when relevant, rather than manufacturing screenshots.

## Initial Scrolling Evidence

Measured on October 7, 2026 with Rust 1.99 on an Apple M2 Max, macOS 26.6.2, release optimization, a
100×30-cell buffer, and Criterion's 10-sample short run (100 ms warm-up, 200 ms measurement). The
fixture inserts numbered Unicode lines containing `界 hello world` into an empty source. These
measurements exclude preparation, layout, terminal I/O, and event handling.

| Mode               | 1,000 lines | 100,000 lines |
| ------------------ | ----------- | ------------- |
| Unified, unwrapped | 67 µs       | 79 µs         |
| Unified, wrapped   | 65 µs       | 69 µs         |
| Split, unwrapped   | 68 µs       | 72 µs         |
| Split, wrapped     | 67 µs       | 72 µs         |

Values are Criterion point estimates, rounded. The short unified/unwrapped 100,000-line run has
noticeable variance (73–94 µs confidence interval). A 100-fold document increase does not produce a
proportional steady-frame increase in these fixtures. This does not establish bounds for arbitrary
long lines, terminal backends, first layout, or memory usage. Reproduce with
`cargo bench --bench viewer -- scroll- --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10`.

## Preparation and Resize Evidence

The same machine and short-run settings produced these 100,000-line point estimates. Resize
alternates widths of 99 and 100 cells; cached glyphs and alignment are reused.

| Operation          | Unified, unwrapped | Unified, wrapped | Split, unwrapped | Split, wrapped |
| ------------------ | ------------------ | ---------------- | ---------------- | -------------- |
| First layout       | 146 ms             | 146 ms           | 146 ms           | 153 ms         |
| Resize             | 3.0 ms             | 7.2 ms           | 5.1 ms           | 9.4 ms         |

Document preparation took 16.7 ms for 100,000 lines and 152 µs for 1,000 lines. Comparing 200
completely different lines of 4,096 characters took 3.5 ms. These fixtures establish initial costs,
not worst-case bounds. Allocation counts and peak memory have not been measured.

## Selection Rendering Evidence

Measured on October 9, 2026 with Rust 1.99.0 on the same Apple M2 Max and macOS 26.6.2, using
release optimization, a 100×30-cell buffer, and the short Criterion settings above. Selection covers
the new source from its first line through its final LF. These timings include steady scrolling and
selection painting; they exclude selection validation, text extraction, preparation, and terminal
I/O.

| Mode               | 1,000 lines | 100,000 lines |
| ------------------ | ----------- | ------------- |
| Unified, unwrapped | 68 µs       | 72 µs         |
| Unified, wrapped   | 67 µs       | 72 µs         |
| Split, unwrapped   | 70 µs       | 75 µs         |
| Split, wrapped     | 70 µs       | 74 µs         |

Selection does not add a whole-document scan to steady rendering. Validation and extraction traverse
available source lines; their cost is outside these measurements. Reproduce with

```sh
cargo bench --bench viewer -- selection-scroll- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
```

## Cached Literal Search Evidence

Measured on October 9, 2026 on an Apple M2 Max, macOS, Rust 1.99, release optimization, and a
100×30-cell buffer. The fixture inserts numbered Unicode lines containing `界 hello world` into an
empty source; the literal query `hello` matches every line. Results are prepared before timing.
Criterion uses 10 samples, 100 ms warm-up, and 200 ms measurement. Values are point estimates
rounded to microseconds.

| Mode               | Search, 1,000 lines | Search, 100,000 lines | No search, 100,000 lines |
| ------------------ | ------------------- | --------------------- | ------------------------ |
| Unified, unwrapped | 68 µs               | 74 µs                 | 70 µs                    |
| Unified, wrapped   | 68 µs               | 74 µs                 | 70 µs                    |
| Split, unwrapped   | 70 µs               | 76 µs                 | 73 µs                    |
| Split, wrapped     | 72 µs               | 76 µs                 | 73 µs                    |

A 100-fold source/result increase adds about 4–6 µs in these short steady-frame runs. Rendering uses
binary source/range lookup and visible glyphs; it does not rescan source text for the query. These
measurements exclude query updates, layout, terminal I/O, and navigation. They do not establish
bounds for arbitrary queries, long lines, memory use, or terminal backends. Reproduce with the
following command:

```sh
cargo bench --bench viewer --locked -- scroll \
    --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
```
