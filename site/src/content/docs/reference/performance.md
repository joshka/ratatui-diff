---
title: Performance
description: Where ratatui-diff spends time and memory, and how to keep the event loop responsive.
---

The short version: steady frames cost about the same for 1,000 lines as for 100,000, because
rendering only paints visible rows. Comparison, first layout, and resize scale with the document, so
large inputs belong on a worker or behind a loading state.

## Guidance for hosts

- **Prepare off the event loop.** `DiffDocument::from_text`, `compare`, and `parse` are synchronous.
  So is syntax preparation.
- **Expect the first render and each resize to touch the whole document.** They build glyph and row
  indexes. Later frames reuse them.
- **Cheap per-frame changes:** theme, word highlights, sticky headers, scrolling, and attaching new
  syntax styles reuse layout.
- **Changes that relayout:** width, wrapping, mode, and folding.
- **Search updates scan the retained source** once per query change; rendering reuses the matches.

## Measurements

Apple M2 Max, Rust 1.99, release build, 100×30-cell buffer, October 9, 2026. Values are the range of
two short Criterion runs' mean estimates, split view with wrapping. They show where costs are, not
latency guarantees.

| Operation                   | 1,000 lines  | 100,000 lines |
| --------------------------- | ------------ | ------------- |
| Compare (insertion fixture) | 0.15 ms      | 16 ms         |
| First layout                | 1.2 ms       | 135–159 ms    |
| Resize                      | 0.14 ms      | 17–19 ms      |
| Toggle one file fold        | 0.12–0.19 ms | 15 ms         |
| Search update, both sides   | 0.06–0.08 ms | 10 ms         |
| Steady frame                | 75–80 µs     | 89–177 µs     |
| Hit-test one cell           | 15 ns        | 15 ns         |

Steady frames in the other modes were 68–82 µs at both sizes; split view with wrapping varied more
between runs at 100,000 lines. Closed files redraw in about 23 µs.

Memory is dominated by prepared layout: the 100,000-line Unicode fixture uses about 33 MiB after
comparison and about 465 MiB resident after its first layout, measured after the glyph storage
change in [#21](https://github.com/joshka/ratatui-diff/pull/21). The timing table above predates
that change; the
[performance notes](https://github.com/joshka/ratatui-diff/blob/main/docs/performance.md#prepared-glyph-storage)
compare first layout and resize before and after it.

## Reproduce

```sh
cargo bench --bench viewer --locked -- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
RATATUI_DIFF_MEMORY=1 cargo bench --bench viewer --locked
```

The [performance notes](https://github.com/joshka/ratatui-diff/blob/main/docs/performance.md) list
what each benchmark includes and excludes, the stress fixtures, and the full result tables.
