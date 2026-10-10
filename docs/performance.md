# Performance measurements

The release benchmarks separate source comparison, structured preparation, initial layout, relayout,
fold-state rebuilds, and cached viewport rendering. They make no absolute CI timing guarantee.
Rendering and interaction behavior are unchanged by this measurement work; the existing
[terminal acceptance and visual evidence contracts](testing.md) still apply.

## Reproduce

Run from the repository root, with no competing builds or benchmark processes. Compile first, then
run each measurement sequentially. Retain Criterion confidence intervals and repeat the run before
interpreting small differences.

```sh
cargo bench --bench viewer --locked --no-run
cargo bench --bench viewer --locked -- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
```

Criterion stores machine-readable results under `target/criterion/`. Filters such as `scenarios/`,
`scroll-`, and `resize-` select smaller groups. The short settings above bound exploration; longer
runs are appropriate for assessing a proposed optimization. Shared CI compiles and smoke-runs the
benchmarks through `cargo test --all-targets`; it does not gate absolute timings.

A separate fresh-process resident-memory probe uses the same optimized benchmark executable:

```sh
RATATUI_DIFF_MEMORY=1 cargo bench --bench viewer --locked
```

Repeat that command three times. The probe prints phase RSS in KiB using `ps` on macOS/Linux, along
with public model storage capacities in bytes. Unsupported RSS collection prints `unavailable`. The
environment variable selects the probe instead of Criterion; it is absent during timing runs.

## Measurement boundaries

| Name                                             | Timed work                                                                                                   | Excluded work                                 |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------- |
| `prepare`, `compare-*`                           | Comparison, source copying, model construction/refinement, output destruction                                | Fixture generation, layout                    |
| `validate-structured`                            | Validation/refinement of cloned prepared files                                                               | Comparison, input cloning, output destruction |
| `layout-*`, `initial-*`                          | First render, glyph/alignment preparation, display/source indexes, viewport painting                         | State creation, cache destruction, comparison |
| `resize-*`                                       | Alternating 99/100-cell widths, display/source index rebuild, painting                                       | Initial glyph preparation, comparison         |
| `context-rebuild-*`, `largest-context-rebuild-*` | Toggle one retained-context fold, rebuild display/source indexes, paint                                      | Initial glyph preparation                     |
| `file-rebuild-*`                                 | Toggle one file, rebuild display/source indexes, paint                                                       | Initial glyph preparation                     |
| `scroll-*`, `steady-*`                           | Advance viewport, wrap to start, paint into an existing buffer                                               | Comparison, initial layout, terminal I/O      |
| `selection-update`, `selection-extract`          | Validate/set a full-document selection or extract its source text; include temporary/output text destruction | Layout, painting                              |
| `search-update-*`                                | Search all retained context on both sides and clear results                                                  | Layout, navigation, painting                  |
| `search-scroll-*`, `selection-scroll-*`          | Cached search/selection painting and scrolling                                                               | Query update, selection validation/extraction |

Initial layout uses Criterion's batched setup so destroying the returned state is outside the timed
routine. Earlier layout measurements in [Testing](testing.md) include state destruction and should
not be treated as a directly comparable baseline. Comparison benchmarks retain their existing
end-to-end construction/destruction boundary. Structured preparation starts with already-prepared
input and excludes source comparison/copying. `DiffDocument::new` still runs refinement; on this
core revision it computes word differences even when existing highlight ranges remain authoritative.

All frames use a 100×30-cell buffer. Main viewer fixtures insert 1,000 or 100,000 numbered Unicode
lines. Retained-context fixtures change line 501 and retain all source: each line includes a wide
character, combining mark, ZWJ emoji, and literal `hello`. They exercise unified/split and
wrapped/unwrapped layouts, initially open/closed files, closed/open context, and fold rebuilds. One
file and two context candidates isolate folding costs; they do not establish scaling with the number
of files or folds. Closed overviews can fit within the viewport, so their steady benchmark measures
repeated redraw rather than traversal of 100,000 visible rows.

Bounded stress cases use a 100,000-byte ASCII line, 10,000 repeated Unicode grapheme groups, 128 old
versus 257 new lines, 4,000 alternating repeated lines shifted by one, and the existing 200
completely dissimilar 4,096-character lines. These exercise long-line clipping/wrapping, uneven
split runs, and ambiguous comparisons without claiming worst-case bounds. Long-line
layout/resize/steady cases use split mode; the main and folding fixtures cover both modes.

## Memory methodology

Public capacities distinguish copied source text, `DiffLine` vector backing storage, and prepared
word-highlight vectors. They exclude allocator metadata, file/hunk/path allocations, private layout
and search structures, stacks, executable mappings, and the fixed terminal buffer. The probe retains
one document/state/buffer and drops original input strings before layout.

RSS includes those excluded categories and allocator-retained pages. A phase delta is process-level
resident growth, not an exact allocation count or live cache size. Dropping inputs may not return
pages to the OS. Peak comparison temporaries are not captured by phase snapshots. The probe does not
measure allocation counts or attribute individual private allocations.

Code inspection establishes the categories: prepared logical rows own grapheme glyph vectors;
wrapped screen rows and old/new source indexes are rebuilt for geometry/folding; search retains
matches and source lookup indexes separately. Source word highlights live in the document. RSS
changes can support a cache-growth hypothesis but cannot apportion bytes between glyphs, row/index
vectors, and search structures. Exact attribution would require allocator profiling in a separate
instrumented build, without changing the crate's unsafe-code policy.

## October 9, 2026 baseline

Measured on an Apple M2 Max with 96 GiB RAM, macOS 26.6.2, Rust 1.99.0 (`b940084d7`,
aarch64-apple-darwin), default Cargo release/bench optimization and locked dependencies. The core
revision is `f5a0d42001758541a9df97cbc3723028209fe8fc`, including file/context folding. Related
project builds/checks were paused; each benchmark process ran sequentially. Background desktop
applications remained active, with no CPU affinity, frequency, thermal, or allocator controls.

Tables show the range of the two runs' Criterion **mean** point estimates, rounded, rather than a
confidence interval or a guaranteed latency. Short runs contain outliers and substantial variation
for some layout/resize cases. Use these results to prioritize large costs, not to claim small
regressions or speedups. Criterion can extend measurement beyond 200 ms to collect ten slow samples.

The first run covers 167 cases before the largest-context cases were added; those are measured
separately below. The repeat covers 87 cases using this exact filter and retains independent results
instead of overwriting the first run:

```sh
cargo bench --bench viewer --locked -- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10 \
  --save-baseline performance-1
perf_filter='viewer/(prepare|selection-update|selection-extract|.*scroll-|'
perf_filter+='layout-Split-wrap-true|resize-Split-wrap-true|dissimilar)|'
perf_filter+='scenarios/.*(Split-wrap-true|compare-|validate-|long-|uneven|repeated)'
cargo bench --bench viewer --locked -- "$perf_filter" \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10 \
  --save-baseline performance-2
```

### Preparation, layout, and updates

Times are milliseconds. Layout/folding/search rows below use split mode with wrapping. Insertion
rows use the main viewer fixture; retained rows use the single-change Unicode context fixture. File
rebuilds alternate open/closed with both context folds expanded. The first-context rebuild toggles
the 497-line leading fold; the largest-context measurement below covers the long tail.

| Operation                            | 1,000 lines (ms) | 100,000 lines (ms) |
| ------------------------------------ | ---------------- | ------------------ |
| Compare insertion                    | 0.150–0.153      | 16.054–16.343      |
| Initial insertion layout             | 1.242–1.257      | 134.832–158.925    |
| Resize insertion                     | 0.141–0.142      | 17.109–19.045      |
| Compare retained context             | 0.275–0.302      | 29.382–40.097      |
| Validate prepared structured input   | 0.009–0.009      | 0.860–0.881        |
| Initial retained layout, file open   | 2.351–2.731      | 246.492–266.713    |
| Initial retained layout, file closed | 2.342–2.659      | 253.363–268.724    |
| Resize, all context open             | 0.202–0.205      | 28.855–53.559      |
| First context rebuild, file open     | 0.107–0.169      | 0.968–1.063        |
| First context rebuild, file closed   | 0.041–0.042      | 1.497–1.572        |
| File rebuild, all context open       | 0.122–0.193      | 15.291–15.711      |
| Search update + clear, both sides    | 0.062–0.078      | 10.204–10.351      |
| Select full inserted source          | 0.008–0.008      | 0.793–0.799        |
| Extract full inserted source         | 0.008–0.008      | 0.777–0.835        |

The largest-context runs independently toggle the longest candidate (99,496 lines in the 100k
fixture), with the other candidate closed. Repeat the following command with `largest-2` to retain
the second run:

```sh
cargo bench --bench viewer --locked -- largest-context-rebuild-Split-wrap-true \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10 \
  --save-baseline largest-1
```

| Largest context rebuild | 1,000 lines (ms) | 100,000 lines (ms) |
| ----------------------- | ---------------- | ------------------ |
| File open               | 0.104–0.106      | 15.232–16.829      |
| File closed             | 0.042–0.044      | 1.456–2.779        |

### Steady viewport rendering

Times are microseconds; preparation/layout and search/selection updates are excluded.

| Mode               | Plain 1k | Plain 100k | Search 1k | Search 100k | Selection 1k | Selection 100k |
| ------------------ | -------- | ---------- | --------- | ----------- | ------------ | -------------- |
| Unified, unwrapped | 68–82    | 72–72      | 70–89     | 76–78       | 68–70        | 73–74          |
| Unified, wrapped   | 68–68    | 72–73      | 69–70     | 76–82       | 68–69        | 72–117         |
| Split, unwrapped   | 70–71    | 75–75      | 72–74     | 77–80       | 71–71        | 76–200         |
| Split, wrapped     | 75–80    | 89–177     | 72–82     | 78–80       | 72–94        | 95–121         |

| Retained context, split/wrapped | 1,000 lines (µs) | 100,000 lines (µs) |
| ------------------------------- | ---------------- | ------------------ |
| File closed                     | 22–22            | 22–23              |
| File open, context closed       | 47–76            | 46–46              |
| File and context open           | 101–103          | 109–117            |

A 100-fold source increase produces no proportional steady-frame increase in these cases. Closed
file redraw stays around 23 µs. Code inspection agrees: an unchanged layout/fold key returns before
source/index rebuilding, and painting visits visible screen rows. Cached file summaries avoid
per-frame source statistics scans. These fixtures support viewport-sized steady work; they do not
establish asymptotic bounds for every document, query, terminal backend, or number of files/folds.

### Bounded stress cases

Times are milliseconds. Layout/resize/steady use split mode; paired values show unwrapped / wrapped.

| Fixture                        | Comparison  | Initial layout            | Resize                    | Steady frame              |
| ------------------------------ | ----------- | ------------------------- | ------------------------- | ------------------------- |
| 100k-byte ASCII line           | 0.069–0.069 | 4.137–4.375 / 4.536–4.576 | 0.027–0.043 / 0.165–0.332 | 0.025–0.033 / 0.087–0.087 |
| 10k Unicode groups             | 0.105–0.116 | 2.253–3.506 / 2.353–2.388 | 0.027–0.028 / 0.121–0.123 | 0.026–0.026 / 0.100–0.102 |
| 128 deletions / 257 insertions | 0.371–0.663 | 0.213–0.361 / 0.252–0.411 | 0.072–0.083 / 0.075–0.107 | 0.053–0.121 / 0.052–0.113 |
| Shifted 4k repeated lines      | 0.223–0.234 | 0.040–0.042 / 0.040–0.041 | 0.034–0.037 / 0.034–0.035 | 0.040–0.049 / 0.032–0.032 |

The 200 dissimilar 4,096-character-line comparison takes 3.96–4.23 ms. The shifted repeated-line
case has a small edit after comparison; its low layout cost does not represent displaying 4,000
replacement rows. Uneven/long-line samples contain outliers; precise relative rankings need longer
runs. Comparison remains synchronous and is not bounded by the word-refinement limits.

### Resident memory

Three fresh-process probes use the retained 100,000-line fixture in split/wrapped mode. Values below
are the observed RSS range in MiB (KiB / 1,024), not live owned heap totals.

| Phase                      | RSS (MiB)   | Displayed rows |
| -------------------------- | ----------- | -------------- |
| `input`                    | 10.2–10.3   | 0              |
| `prepared_with_inputs`     | 33.1–34.3   | 0              |
| `prepared_inputs_dropped`  | 33.1–34.3   | 0              |
| `initial_file_closed`      | 648.0–651.2 | 1              |
| `file_open_context_closed` | 648.0–651.2 | 11             |
| `context_open`             | 649.0–652.2 | 100,002        |
| `search_all_lines`         | 665.7–667.1 | 100,002        |
| `after_1000_frames`        | 665.8–667.1 | 100,002        |

Accessible document capacities are 3,688,928 bytes of source text, 11,534,336 bytes of line-vector
storage, and 128 bytes of word highlights: approximately 14.5 MiB combined. Original input string
capacities total 8,114,269 bytes before being dropped. These are storage capacities, not allocation
counts or full document totals.

The first layout adds roughly 615–617 MiB RSS even with the file closed and only one displayed row.
Opening all context adds about 1 MiB; a literal matching all context lines adds about 15–17 MiB.
After 1,000 frames the additional observed growth is only 48–64 KiB. These snapshots do not prove
absence of leaks or measure peak transient memory. RSS remaining unchanged after dropping inputs is
consistent with allocator page retention; exact attribution needs an allocator profile.

## Prepared glyph storage candidate

The October 9 baseline above remains unchanged. A follow-up on immutable main
`2983b0b5f442cbecfe2985ffe4fdcd5231cf8d34` replaces each glyph's owned string with a range into one
prepared line buffer. Ordinary graphemes index its source prefix; tabs, whitespace markers, control
escapes, dotted-circle bases, and newline notation append display text. Original source ranges
remain independent. Completed glyph arrays use exact-size boxed slices, discarding vector capacity
reserved during preparation. Split context still prepares both sides, including hidden files; this
change does not defer preparation or change display/source index rebuilding.

Safe temporary instrumentation of the existing 100,000-line retained Unicode fixture accounts for
private storage structurally. It sums glyph vector capacities times `size_of::<Glyph>()`, text
capacities, and logical-row vector capacity times `size_of::<Row>()`. The candidate sums boxed slice
lengths instead of vector capacities. These figures exclude allocator metadata, alignment rounding,
temporary allocations, display/source indexes, and other state. No allocator profiler was used.

| Attributed retained category | Baseline        | Candidate       |
| ---------------------------- | --------------- | --------------- |
| Live glyphs                  | 4,577,827       | 4,577,827       |
| Bytes per glyph              | 72              | 64              |
| Glyph backing bytes          | 460,804,608     | 292,980,928     |
| Display text capacity bytes  | 7,377,834       | 7,377,834       |
| Nonempty text allocations    | 4,577,827       | 200,002         |
| Logical-row backing bytes    | 27,262,976      | 31,457,280      |

The attributed categories decrease by 163,629,376 bytes (156.0 MiB). Baseline glyph vector slack
alone accounts for 131,201,064 bytes (125.1 MiB). These sums establish representation costs; they do
not explain every RSS page or quantify allocator overhead from the removed small allocations.

Measurements use the same machine, locked dependencies, release settings, terminal buffer, and
retained fixture as the baseline. Separate optimized baseline/candidate executables run sequentially
while related builds and terminal captures are paused. Two uncontended Criterion runs per executable
retain independent baselines; an earlier potentially overlapping baseline run is excluded. Each
executable also runs three fresh-process memory probes. Short-run mean ranges and outliers limit
claims about small timing changes.

```sh
glyph_filter='scenarios/(compare-retained|initial-Split-wrap-true-file-closed-(false|true)|'
glyph_filter+='context-open-resize-Split-wrap-true|context-open-steady-Split-wrap-true)/100000$'
cargo bench --bench viewer --locked -- "$glyph_filter" \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10 \
  --save-baseline glyph-candidate-1
RATATUI_DIFF_MEMORY=1 cargo bench --bench viewer --locked
```

Repeat with a distinct baseline name, and run the memory probe three times. Compare the immutable
baseline in a separate jj workspace; never change an active owner's revision for measurements.

| Operation, retained 100k split/wrapped | Baseline mean range | Candidate mean range |
| -------------------------------------- | ------------------- | -------------------- |
| Source comparison                      | 28.96–31.48 ms      | 27.11–29.46 ms       |
| Initial layout, file open              | 243.26–254.07 ms    | 176.52–186.79 ms     |
| Initial layout, file closed            | 244.60–264.58 ms    | 176.15–179.42 ms     |
| Resize, all context open               | 30.09–30.50 ms      | 21.78–22.36 ms       |
| Cached frames, all context open        | 109.9–113.4 µs      | 106.0–110.5 µs       |

Source preparation is unchanged; its overlapping ranges provide a comparison control. Resize still
rebuilds display/source indexes. Its observed decrease does not establish an index optimization; the
smaller retained representation can affect traversal and allocator behavior. Cached rendering shows
no large change in these short runs.

| Fresh-process RSS phase   | Baseline (MiB) | Candidate (MiB) |
| ------------------------- | -------------- | --------------- |
| Prepared, inputs dropped  | 33.5–34.7      | 33.1–34.5       |
| Initial file closed       | 648.2–651.3    | 463.4–464.6     |
| File open, context closed | 648.2–651.3    | 463.4–464.6     |
| All context open          | 649.3–652.3    | 468.0–469.3     |
| Search all lines          | 665.9–666.5    | 472.1–474.2     |
| After 1,000 frames        | 666.0–666.6    | 472.1–474.2     |

Closed-file first-layout RSS decreases by about 28.5% in this fixture. RSS remains above attributed
live capacities and includes retained allocator pages. Phase deltas, including the smaller observed
search delta, are not exact private cache sizes. Peak resident memory and allocation lifetimes
remain unmeasured. Unicode/source, clipping, wrapping, split pairing, folding, search, selection,
reveal, and replacement contracts remain covered by focused and existing integration tests.

## Baseline follow-up units

These recommendations describe the original measurement-only baseline. The prepared glyph storage
candidate above implements its first investigation; the remaining units are independent.

1. Profile glyph allocations first. Baseline initial layout prepares every retained source grapheme,
   including hidden files/context; ordinary graphemes own individual `String`s, and split context
   prepares both panes. Test source-backed glyph ranges with owned text only for transformations, or
   deferred glyph preparation for closed regions, as separate changes. Preserve Unicode geometry,
   synthetic cues, source hits, reveal, search, and selection; require matched visual and terminal
   evidence. The RSS result motivates investigation, not an exact byte attribution or a promised
   saving.
1. Profile display/source index rebuilding before changing folding/resize. Open-context resize and
   file toggles are milliseconds even though steady painting is cheap. Investigate reusing unchanged
   indexes or avoiding repeated work in hidden regions; preserve source reveal and navigation before
   claiming a win. Do not add a broad cache abstraction based only on these runs.
1. Separate selection validation from text extraction. Setting a full-source selection currently
   constructs and discards the selected text, so the update costs roughly the same as extraction. A
   validation-only path should retain gap/grapheme/end-of-line checks and be measured independently.
1. Use the insertion and uneven baselines when evaluating replacement-pairing changes. Their
   preparation and initial-layout boundaries expose costs separately; cached scrolling should remain
   unaffected. Evaluate candidate code only from an immutable coordinator snapshot.

The original baseline includes no core optimization or CI timing gate. Private allocator
attribution, peak RSS, many-file/many-fold scaling, terminal output latency, and candidate alignment
performance remain unmeasured.

## Folding and resize profiles after glyph storage

[The folding and resize report](folding-resize-performance.md) profiles shipped `b5d6930c` with many
files and context folds. It separates state mutation, first redraw, narrow resize, and warm frames,
then attributes transition costs with recorded stacks. Its evidence and bounded follow-up
investigations supplement the historical measurements above; it includes no runtime optimization.
