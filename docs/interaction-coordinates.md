# Interaction coordinates

`SourcePosition` identifies a file index, old/new side, and one-based source line. `SourceRange`
adds a half-open UTF-8 byte range within that line's text, excluding its line ending. Coordinates
belong to the document that produced them; hosts must discard them on replacement. Missing patch
context has no source coordinate. File-wide byte offsets cannot be reconstructed.

`DiffState::hit_test(x, y)` uses absolute terminal cells from the last render. It returns `None`
before rendering, outside the viewport, or after navigation until the next render. Hosts should call
`invalidate_hit_testing` when a resize or document/options replacement occurs before drawing.
Rendering refreshes the mapping, including viewport origin, size, wrapping, and scrolling.

Source hits return the complete grapheme byte range. Every expanded tab cell maps to the tab byte;
wide grapheme cells and every cell of a control escape map to their complete source range. Combining
characters remain with their grapheme; standalone combining clusters include their synthetic base
only in display space. Partially clipped or oversized glyphs are blank padding, without source.
Unified context hits contain both old and new ranges. Split hits contain only their pane's side.

Headers, gutters, pane separators, padding, and final-newline notation are distinct hit results.
Gutters and final-newline cues carry available line identities but no source bytes. Empty lines have
line identities through their gutter, but no source glyph. Missing split partners and exhausted
wrapped segments have no source hit. Trailing pane padding and the spare even-width cell are
padding.

Selection can retain source ranges independently of display geometry. Only rendered cells map to
source; there is no inverse mapping for omitted context or an assumption that every source byte is
currently visible. Hosts own pointer events, clipboard access, repository I/O, and terminal setup.

## Host demonstration

Run `cargo run --example viewer -- --aardvark-ink`. Move the mouse over source text to inspect its
coordinates, or press `p` to probe a fixed content cell. The status row prefers the new side when
both are present; applications can inspect both ranges returned by the API. The example owns mouse
capture and event handling. `examples/hit-testing.tape` captures the fixed probe with Betamax.

## Lookup cost

On October 9, 2026, Apple M2 Max, Rust 1.99, release optimization, a 100×30-cell viewport, and
Criterion's 10-sample run (100 ms warm-up, 200 ms measurement), source-cell lookup took 14–15 ns
across unified/split and wrapped/unwrapped views for both 1,000 and 100,000 lines. The fixture
inserts numbered Unicode lines containing `界 hello world`. Reproduce with
`cargo bench --bench viewer -- hit- --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10`.

Steady scrolling measured 67–70 µs for 1,000 lines and 71–77 µs for 100,000 lines. These short runs
support viewport-scale drawing and document-size-independent lookup for this fixture. They do not
establish worst-case latency for long grapheme sequences. Retaining source byte ranges increases
glyph-cache memory; allocation counts and peak memory have not been measured.
