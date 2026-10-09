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

## Visual Evidence During Development and Review

For visible changes, show what is being built throughout development. Capture the baseline before
editing, then capture each meaningful result with Betamax and inspect the images. Include labeled
before/after images in Codex commentary as changes occur, in the final handoff, and in the PR body.
A reviewer should be able to identify the improvement from a couple of images and a short caption.
Do not substitute a list of styling changes or passing tests for visual evidence.

Use the same fixture, terminal dimensions, font, theme, and viewport position for each comparison.
Label intentional differences. Choose views that reveal the change: unified/split, narrow/wrapped,
search/selection states, or metadata as appropriate. Use static images for close inspection; include
an animation only when motion or an interaction sequence matters. Capture intermediate states when
they explain an interaction. Show rejected experiments when they clarify a meaningful tradeoff.

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
