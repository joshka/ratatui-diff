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
