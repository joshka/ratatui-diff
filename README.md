# ratatui-diff

<!-- rumdl-disable MD013 -->
<!-- cargo-rdme start -->

A navigable unified and split diff widget for Ratatui.

Display text comparisons or Git patches with line numbers, word highlights, and keyboard-driven
navigation. Your application supplies terminal setup, key bindings, and input data. Rendering
depends only on `ratatui-core`.

## Render in a Ratatui application

Prepare a [`DiffDocument`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html) once and keep a [`DiffState`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html) between frames to retain the viewport
and cached layout. Build a [`Diff`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.Diff.html) in the draw closure to select presentation options:

```rust
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui_diff::{Diff, DiffDocument, DiffState, ViewMode};

let document = DiffDocument::from_text("hello world\n", "hello Rust\n");
let mut state = DiffState::new();
let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
terminal.draw(|frame| {
    let diff = Diff::new(&document).mode(ViewMode::Split);
    frame.render_stateful_widget(&diff, frame.area(), &mut state);
})?;
// Call from your PageDown handler, then redraw.
state.scroll_pages(1);
```

This example uses an in-memory terminal. The draw closure also works with Crossterm or Termion.
The [interactive viewer] includes key bindings, an event loop, and terminal restoration.

[interactive viewer]: https://github.com/joshka/ratatui-diff/blob/main/examples/viewer.rs

## Choose an input

- [`DiffDocument::from_text`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html#method.from_text) compares old/new UTF-8 source with three context lines.
- [`DiffDocument::compare`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html#method.compare) selects the number of context lines when generating hunks.
- [`DiffDocument::parse`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html#method.parse) reads unified or multi-file Git patches; errors include byte offsets
  when available.
- [`DiffDocument::new`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html#method.new) validates caller-provided files, hunks, lines, and highlight ranges.

```rust
use ratatui_diff::DiffDocument;

let patch = "--- a/greeting\n+++ b/greeting\n@@ -1 +1 @@\n-hello world\n+hello Rust\n";
let document = DiffDocument::parse(patch)?;
assert_eq!(document.files().len(), 1);
```

A patch displays only its supplied changes and context. Binary changes appear as
summaries. Combined merge diffs are rejected. Source content retains CRLF and final-newline
differences; control characters are displayed as visible text.

## Configure appearance

The defaults are unified view, visible line numbers and word highlights, no wrapping or
whitespace markers, four-cell tab stops, and the dark theme. Split replacements pair lines in
source order and pad the shorter side. Each [`DiffTheme`](https://docs.rs/ratatui-diff/latest/ratatui_diff/theme/struct.DiffTheme.html) style can be customized independently.

```rust
use ratatui_diff::{Diff, DiffDocument, DiffTheme, ViewMode};

let document = DiffDocument::from_text("old\n", "new\n");
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .theme(DiffTheme::light())
    .wrap(true)
    .whitespace(true)
    .tab_width(4);
```

Wrapped split rows use the taller side's height. Without wrapping, horizontal scrolling is
synchronized across panes. Graphemes are never split at viewport edges. Disable inline
highlights with [`Diff::word_highlights`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.Diff.html#method.word_highlights) when whole-line styling is sufficient.

## Navigate and retain the viewport

Render once before page navigation or source mapping so [`DiffState`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html) has the viewport
dimensions and row positions. Scrolling counts displayed rows, including headers and wrapped
continuations. Source positions identify a file, an old or new side, and a one-based line
number.

- Lines, pages, half pages: [`scroll_lines`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.scroll_lines),
  [`scroll_pages`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.scroll_pages),
  [`scroll_half_pages`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.scroll_half_pages).
- Horizontal cells: [`scroll_horizontal`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.scroll_horizontal).
- Start/end: [`start`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.start), [`end`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.end).
- Hunk/file boundaries: [`next_hunk`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.next_hunk),
  [`previous_hunk`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.previous_hunk), [`next_file`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.next_file),
  [`previous_file`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.previous_file).
- Source positions: [`scroll_to_source`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.scroll_to_source),
  [`source_at`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.source_at).

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, Side, SourcePosition};

let document = DiffDocument::from_text("before\n", "after\n");
let area = Rect::new(0, 0, 80, 3);
let mut buffer = Buffer::empty(area);
let mut state = DiffState::new();
(&Diff::new(&document)).render(area, &mut buffer, &mut state);
assert!(state.scroll_to_source(SourcePosition {
    file: 0,
    side: Side::New,
    line: 1
}));
```

Resizing or switching modes retains the nearest available source anchor. Replacing the document
resets navigation on the next render. Theme and word-highlight changes preserve layout. Give
each independently navigated pane its own state, even when both borrow the same document.

## Select source for copying

Keep anchor and focus in [`DiffState::set_selection`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.set_selection), then extend focus with
[`SelectionMotion`](https://docs.rs/ratatui-diff/latest/ratatui_diff/selection/enum.SelectionMotion.html) from host key bindings. Pointer handlers can convert a hit [`SourceRange`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.SourceRange.html)
with [`start_boundary`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.SourceRange.html#method.start_boundary) and
[`end_boundary`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.SourceRange.html#method.end_boundary). Selection stays within one file and side, survives
resizing and view changes, and clears on document replacement.

[`selected_text`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html#method.selected_text) returns original source bytes for the host's
clipboard integration. It preserves tabs, controls, CRLF, and final-newline state; it excludes
gutters, wrapping, padding, and display notation. Invalid grapheme boundaries and missing patch
context are rejected. [`Diff::selection_style`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.Diff.html#method.selection_style) controls the overlay applied after word
highlights.

## Large documents

Comparison is synchronous; prepare large documents outside the event loop. Rendering reuses the
comparison results and draws only visible rows, but first layout and resize can process the
whole document. Large replacements fall back to whole-line styling when they exceed the
word-comparison size limits.

## Compatibility

This pre-1.0 API may change between minor releases.

<!-- cargo-rdme end -->
<!-- rumdl-enable MD013 -->

## Views

Unified view keeps additions and deletions together, with changed words emphasized.

![Unified diff with line numbers and word highlights](https://github.com/joshka/ratatui-diff/releases/download/media-v1/unified.png)

Split view places old source on the left and new source on the right.

![Split diff with aligned old and new source](https://github.com/joshka/ratatui-diff/releases/download/media-v1/split.png)

[Whitespace markers](https://github.com/joshka/ratatui-diff/releases/download/media-v1/whitespace.png)
make spaces visible. The
[animated demo](https://github.com/joshka/ratatui-diff/releases/download/media-v1/viewer.gif) holds
each captioned view for five seconds.

## Development

See [Contributing](CONTRIBUTING.md), [architecture](docs/architecture.md),
[dependency decisions](docs/dependencies.md), and [deferred work](docs/roadmap.md). Run
`just example` for the interactive viewer. See [releasing](docs/releasing.md) for package and
screenshot publication.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
