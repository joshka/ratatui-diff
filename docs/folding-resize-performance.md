# Folding and Resize Performance

[Download the raw measurements, profiles, and capture evidence](https://github.com/joshka/ratatui-diff/releases/download/review-media-2026-10-09/folding-resize-evidence-20261009.tar.gz).

The maintainer driver `examples/folding_profile.rs` separates fold state changes from the following
render. It changes no library behavior and is intentionally included with the packaged examples so
maintainers can reproduce profiling against a published source package. It is not a host integration
example.

## Reproduction

Build in an isolated target directory with release optimization and debug symbols:

```sh
CARGO_TARGET_DIR=target/folding-profile CARGO_PROFILE_RELEASE_DEBUG=1 \
  cargo build --release --example folding_profile --locked
python3 scripts/folding-profile/run.py \
  target/folding-profile/release/examples/folding_profile media/folding-profile/results.csv
```

Run quantitative measurements with other builds, tests, and captures stopped. The driver reports
arithmetic means from serial repetitions; these are exploratory measurements, not Criterion
confidence intervals or timing gates. Initial layout excludes fixture construction and destruction
of the previous state. State transitions exclude rendering. First renders exclude the triggering
state mutation. Resize alternates widths of 100 and 40 cells at height 30 and includes rendering.
Steady rendering uses the same viewport and buffer without terminal output. Navigation requests the
last retained new source line without rendering the resulting reveal. Each invocation constructs a
fresh document and warms its initial layout before the measured phase.

The fixture retains all context, changes every hundredth line, and includes wide characters,
combining marks, and ZWJ emoji. The matrix covers 1,000 and 100,000 lines in one file and 100 files
with 1,000 lines each. Unified/split, wrapping, collapsed/expanded files, and collapsed/open context
are separate cases. `resize-open` and `steady-open` expand all context candidates, including those
inside collapsed files. File/context first-render cases alternate one handle's expansion; bulk
fold-all mutation costs are outside this matrix.

```sh
RATATUI_DIFF_PROFILE_CHECK=1 \
  target/folding-profile/release/examples/folding_profile 2 1000 context-first split wrap closed 2
samply record --save-only --unstable-presymbolicate -o media/folding-profile/resize.json.gz \
  target/folding-profile/release/examples/folding_profile 1 100000 resize split wrap expanded 500
```

The check mode validates fixture handles without reporting a quantitative result. Profiles include
fixture setup and initial preparation; attribute transition work using the repeated operation's
stacks rather than whole-process totals. Profile output and captures are ignored generated media.

## October 9, 2026 Measurements

Measured shipped main `b5d6930c` on an Apple M2 Max, macOS Darwin 25.6.0, Rust 1.99.0, release
optimization with debug symbols, and a 100×30-cell buffer. The coordinator stopped the other two
tasks' builds and measurements for the exclusive window, 03:17:54–03:21:16 UTC on October 10 (202
seconds; October 9 local time). There were no builds or captures during the window. The driver
executable SHA-256 was `b69b793c4ca41705dedc355bc3cb3495e2eb6e0d9b6a1228f7c6f4f50232b334`.

The tracked [evidence manifest](../scripts/folding-profile/evidence-20261009.json) records the
immutable baseline, executable/source hashes, profile arguments, selected windows, and SHA-256
digests for the raw results and captures. It identifies archived evidence independently of this
checkout's machine-specific directory. Generated files must accompany the manifest when sharing
these results.

The complete 240-case raw matrix lives in `media/folding-profile/results.csv`. Values below are
rounded arithmetic means in milliseconds for one file with 100,000 retained lines and 1,001 context
fold candidates. Files are expanded; context starts collapsed. `Open resize` expands all context.
Initial layout uses three repetitions; transitions and frames use twenty.

| Mode    | Wrap | Initial | Context redraw | File redraw | Resize | Open resize | Warm frame |
| ------- | ---- | ------- | -------------- | ----------- | ------ | ----------- | ---------- |
| Unified | Off  | 216     | 2.47           | 2.63        | 2.44   | 5.39        | 0.292      |
| Unified | On   | 199     | 4.03           | 3.36        | 3.90   | 11.79       | 0.103      |
| Split   | Off  | 377     | 2.85           | 2.69        | 2.50   | 6.99        | 0.149      |
| Split   | On   | 393     | 4.73           | 3.86        | 4.78   | 24.91       | 0.111      |

With the file initially collapsed, split/wrapped initial layout still took 398 ms, context redraw
2.63 ms, resize 2.76 ms, and a warm frame 0.022 ms. Hiding a file reduces displayed-row work but
neither initial source-glyph preparation nor source indexing disappears. Opening context inside a
closed file does not make the file visible. State-only file/context changes averaged 0.023/0.091 µs
and repeated source-navigation requests 0.202 µs in this case, using 10,000 repetitions. Such tiny
means include clock overhead and should only establish that mutation is far smaller than redraw. The
first already-collapsed file request is a no-op in closed-file transition averages.

For comparison, split/wrapped initial layout with one expanded 1,000-line file took 4.40 ms, resize
0.124 ms, open-context resize 0.292 ms, and warm rendering 0.111 ms. With 100 expanded files of
1,000 lines each (1,100 fold candidates), those costs were 376 ms, 4.97 ms, 27.23 ms, and 0.112 ms.
File redraw took 5.98 ms in that many-file case. At equal retained line counts, splitting the input
into 100 files does not remove whole-document transition work. These fixtures do not isolate file
count from fold count or prove a general complexity bound.

Short-run outliers include the 0.292 ms unified/unwrapped warm frame and a 14.83 ms many-file
split/unwrapped context redraw. Do not interpret every cell as a stable regression threshold. The
matrix has no confidence intervals and mixes viewport contents across phases. It measures no
terminal transport, event processing, allocation count, peak memory, or height-only resize. Mode
switch, search-driven and selection-driven first renders, long single-line inputs, and bulk fold-all
mutation remain separate coverage gaps. Existing consumer tapes validate the source contracts for
these interactions; they do not measure their latency.

## Stack Attribution

Samply 0.13.1 recorded five 1 kHz profiles with saved symbol sidecars. Transition profiles run 500
repetitions; hidden initial layout runs twenty. Profile timings are sampling-instrumented and are
not substituted for the uninstrumented matrix. `scripts/folding-profile/summarize.py` counts outer
native symbols. Inline frames saved on a symbol-table entry describe a representative address and
must not be assigned to every address in that symbol. Compiler-folded generic symbol names also do
not reliably identify the allocated element type.

The transition summaries select interior time windows after initial preparation and before final
teardown. Percentages are inclusive sample shares, overlap, and must not be summed. Each selected
window contains only render stacks; it has no fixture-comparison or initial source-construction
samples. Raw profiles, sidecars, whole-process summaries, and window summaries are retained
together.

| Profile        | Selected time (ms) | Samples | Prepare | Content | Segments | Source sort |
| -------------- | ------------------ | ------- | ------- | ------- | -------- | ----------- |
| Resize         | 1500–3000          | 1470    | 98.1%   | 22.0%   | 41.0%    | 2.7%        |
| Open resize    | 3000–13000         | 9262    | 99.6%   | 4.0%    | 72.2%    | <1%         |
| Hidden context | 1000–1900          | 888     | 99.1%   | 35.7%   | <1%      | 4.6%        |
| Many-file fold | 1000–1700          | 700     | 95.7%   | 39.1%   | 6.0%     | 3.7%        |

The first three profiles use one 100,000-line split/wrapped file. The fourth toggles the first file
of 100×1,000 retained lines in unified/unwrapped mode. The full hidden-initial profile has 8,435
samples: `content` accounts for 92.5%, grapheme iteration 48.3%, and allocation growth 19.5%.
Initial glyph preparation remains the largest cold cost even when the first visible result is only a
file header.

`Diff::prepare` reuses logical rows for fold and width transitions. `index_screen_rows` nevertheless
recreates every file-header and fold-summary glyph sequence before deciding which rows are hidden.
The transition profiles' `content` and grapheme samples therefore identify synthetic-control
preparation, not regeneration of every retained source line. The hidden-context profile also spends
23.4% of samples in a file-index hash lookup. The source loop tests collapsed-file membership for
each retained row. Every retained source line still receives an index entry pointing to its file
header or context control. Sorting those source indexes is measurable but is a smaller share than
control preparation and membership lookup in these profiles.

When context is open and wrapping enabled, `segments` dominates resize. Allocation growth appears in
33.2% of the selected open-resize samples, overlapping the segment work. The implementation builds
left/right segment vectors per displayed logical row before constructing screen rows. This profile
supports investigating that allocation boundary; it does not establish that glyph caches are
missing.

Rendering can call `prepare` twice when a search or selection reveal opens content after the first
preparation. This follows directly from `render` and `expand_source`; the current profiles do not
measure that combined path. Fold/resize profiles alone are insufficient to justify changing reveal
ordering or broadening cache lifetimes.

## Small Follow-up Units

1. Retain synthetic file-header and fold-summary glyphs when their label and presentation inputs are
   unchanged. Profile hidden-context redraw before and after, where source glyphs are already
   reused. Preserve narrow-label fallbacks, hit invalidation, fold identities, and source reveal.
1. Avoid repeated per-source-row collapsed-file membership checks by carrying the current file's
   expansion through the ordered row walk. Validate many-file navigation, hidden hunk indexes,
   source extraction, and collapse at the viewport. Measure this independently of label reuse.
1. Produce screen rows without allocating two temporary segment vectors per logical row. Measure
   split/wrapped open-context resize first; retain asymmetric pane padding, wide-cell clipping,
   continuation geometry, source boundaries, and selection hit contracts.
1. Profile search/selection reveal during resize with explicit phase markers before considering
   preparation-order changes. Preserve the automatic reveal and manual collapse behavior described
   in [folding controls](folding.md) and [interaction coordinates](interaction-coordinates.md).

These are bounded investigation candidates, not approved runtime changes. No runtime optimization is
included in this profiling change. Source-sort replacement and lazy initial preparation require
stronger evidence and broader contract analysis than these profiles provide.

## Visual and Contract Evidence

Released Betamax 0.1.22 passed `folding` and `files` consumer scenarios against the frozen release
viewer for `b5d6930c`. Its executable SHA-256 is
`4d9a0b645f1f6e25c934975b833264ec1df6b89fe7bc621df145fbc0fdba3736`; its source revision is
`5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2`. The viewer SHA-256 is
`5f67b5934c45e0ed222f3f6f2066c2b65c5c28fa49d9abf7aa9bf7b5597f326a`.

Inspected PNG images under `media/terminal-ux/` show collapsed and expanded retained context,
revealed search source after narrow split/wrapped resize, collapsed file headers, and resized file
content. The paced, captioned `folding-interaction-20261009.gif` and
`file-folding-interaction-20261009.gif` demonstrate the transitions. PNG/JSON assertions validate
rendered output separately from animation readability. No visible runtime change is proposed, so
these are representative baseline states rather than an optimization before/after comparison.

Generated media and profiles stay out of repository history. Preserve `media/folding-profile/` and
`media/terminal-ux/` with the coordinator's durable evidence before workspace cleanup. The profile
hash, fixture parameters, logs, selected windows, and tool identities are necessary for interpreting
the report; the table alone is not the raw evidence.

## Validation

The release driver/viewer build with `--locked`, focused example Clippy with `-D warnings`, nightly
Rust formatting, rumdl, and Markdown lint with the maintainer's 100-column configuration passed.
Eighty check-mode cases exercised all ten phases across unified/split, wrapping, and file expansion
with two 1,000-line files. The raw matrix has 240 unique cases with expected fold counts, and all 57
manifest evidence digests validated. Both Python scripts passed syntax parsing and executed against
the saved evidence. The released-tool `folding` and `files` tapes passed their assertions.

The coordinator owns integration and CI. Broad local `just check`, platform/MSRV suites, package
validation, and hosted CI were not duplicated in this profiling workspace. This change adds
maintainer support and evidence documentation; it changes no runtime API or rendering behavior.
