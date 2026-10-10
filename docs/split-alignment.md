# Split Replacement Alignment

An inserted comment in a replacement run should not put `timeout_ms` beside the comment and
`retries` beside `timeout_ms`. Split layout and automatic word highlights now share a private
pairing function. Similarity chooses ordered anchors; the remaining gaps pair in source order. Every
old and new line occurs once, and each pane retains its original source order.

## Default and Compatibility

This replaces positional pairing by default, without adding a widget setting or public engine API.
Unified row order, source text, numbering, hunk ranges, and explicit caller highlights remain
unchanged. Split row count and opposite-pane partners can change within a replacement run; automatic
highlights change to match those partners in both modes. Hosts should retain source positions rather
than displayed row indexes across layout changes.

Fully dissimilar runs and gaps without anchors retain positional pairing. A first experiment left
all weak matches unpaired, including ordinary one-line replacements; an existing wrapped-coordinate
test exposed that behavior change. Gap completion preserves those replacements while allowing uneven
runs to make room for a related later line. Large runs retain the old policy deliberately. This is a
pre-1.0 behavior improvement, not a configurable compatibility mode.

## Algorithm and Bounds

1. Collect deletion and insertion indexes from one context-free run. Context and hunk boundaries
   stop alignment. One-sided runs pair directly with blanks.
1. Check the run budget before scoring: at most 256 lines and 32 KiB of combined UTF-8 text.
   `deletion_count × insertion_count × run_bytes` must not exceed 1,048,576. This conservative
   estimate bounds candidate bigram-intersection work; it can reject runs whose actual work would be
   lower. Over-budget runs use positional pairing without partial anchors.
1. Trim outer whitespace for scoring only. Build sorted multisets of adjacent Unicode scalar values,
   including an initial sentinel. Duplicate bigrams contribute their minimum occurrence count.
   Integer Dice similarity is `2000 × common / combined_count`; two empty strings score
   1000. Source bytes and graphemes remain untouched.
1. Solve a suffix dynamic program. Matches above 500 earn `similarity − 500`; gaps earn zero.
   Maximize total reward without crossings. Ties prefer pairing, then advancing the old side.
   Repeated equal candidates therefore retain stable earliest partners.
1. Trace anchors from the beginning and zip each intervening gap in source order. Missing partners
   occupy blank panes with no source identity. Use these exact same pairs for refinement.

At the line limit, the dynamic-program tables have at most 16,641 entries each: approximately 81 KiB
combined for scores and choices. Tokens occupy space proportional to bounded run text; sorting costs
at most `O(B log B)` for `B` scalar values. Candidate intersections have the stated conservative
budget, and the dynamic program costs `O(deletions × insertions)`. Collecting indexes and emitting
the fallback still traverse the run. These bounds do not bound whole-document comparison, glyph
preparation, or first layout.

Refinement retains its separate combined 8 KiB / 2,048-word pair limits and 256-line run limit.
Existing explicit highlight ranges, including empty lists, remain authoritative. An unmatched line
receives no automatic ranges and retains whole-line styling. There is no moved-line detection,
syntax interpretation, dependency addition, or score during steady rendering. Alignment is prepared
during document refinement and when layout source rows are rebuilt; resize reuses those rows.

## Compared Approaches

`python3 scripts/compare-alignment.py` compares positional pairing, bounded monotonic greedy
best-match anchoring, and bounded global anchoring. Both candidates use the same similarity, budget,
and gap completion. The experiment counts expected partners retained, not elapsed time. Expected
partners are review judgments for these original fixtures, or the explicit fallback policy for
large/dissimilar runs. Public Rust tests independently exercise production behavior.

| Case              | Source order | Greedy | Global anchors |
| ----------------- | ------------ | ------ | -------------- |
| Inserted comment  | 0/4          | 4/4    | 4/4            |
| Uneven removal    | 1/2          | 2/2    | 2/2            |
| Repeated lines    | 0/2          | 2/2    | 2/2            |
| Unicode           | 0/1          | 1/1    | 1/1            |
| Greedy trap       | 2/2          | 0/2    | 2/2            |
| Dissimilar        | 2/2          | 2/2    | 2/2            |
| Long fallback     | 2/2          | 2/2    | 2/2            |

The greedy trap contains two similar rules. Choosing the later exact copy for the first old rule
prevents the second rule from finding a partner. The global approach retains two good ordered
partners. It adds a small bounded score table, whereas greedy needs no table; both evaluate
candidate similarity. Neither establishes semantic correspondence for arbitrary source. Common
boilerplate, short lines, and repeated punctuation can still attract plausible but unhelpful
anchors. The 500 threshold is a fixed heuristic evaluated on these fixtures, not a universal
accuracy claim.

Runtime scaling measurements belong to the coordinated performance report; the pairing experiment
does not claim to measure Rust latency. The public tests cover byte-, line-, and work-budget
fallbacks, Unicode/source hits, exact selection extraction, search, wrapping, resizing, structured
input, parsed patches, explicit highlight preservation, file/context reveal, repeats, and uneven
replacements.

## Measured Preparation Cost

Two sequential runs per revision compared the merged folding baseline with this alignment candidate
on an M2 Max, Rust 1.99.0, using a 100×30-cell split/wrapped buffer. Known project builds and
captures were paused. Criterion used ten samples, 100 ms warm-up, and 200 ms measurement. These
short runs identify larger costs; they do not establish maximum latency or precise small
regressions.

A 16-deletion/20-insertion run just below the candidate-work budget increased preparation from 29–31
µs to 90–95 µs and first layout from 246–258 µs to 303–309 µs. The added preparation cost is about
60–65 µs; similarity is not free. Over-budget 32/128-line preparation stayed near baseline. Cached
100,000-line insertion frames measured 73–75 µs before and 74–75 µs after.

Some fallback-layout, resize, and 1,000-line steady samples had outliers or inconsistent repeated
results. They cannot support a sustained regression or speedup claim. Alignment runs outside cached
painting, but the cause of those noisy samples was not measured. The
[measurement archive](https://github.com/joshka/ratatui-diff/releases/download/review-media-2026-10-09/alignment-performance-20261009.tar.gz)
contains the report, immutable source provenance, identical harness, commands, logs, and raw
estimates.

## Matched Visual Evidence

`examples/alignment-before.tape` and `examples/alignment-after.tape` capture the same original
`alignment` fixture in split mode: 1458×418 pixels, 22-pixel JetBrains Mono, 22-pixel padding, and
Aardvark Ink. Run the before tape against the original implementation with the fixture added; run
the after tape against the changed implementation. Generated PNG files stay ignored in `media/`. The
comment gets an empty opposite pane, all four settings align, and word emphasis follows the actual
setting pairs. Interaction semantics are unchanged, so these static captures need no GIF.
