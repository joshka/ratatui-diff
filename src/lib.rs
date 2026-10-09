//! A navigable unified and split diff widget for Ratatui.
//!
//! Display text comparisons or Git patches with line numbers, word highlights, and keyboard-driven
//! navigation. Your application supplies terminal setup, key bindings, and input data. Rendering
//! depends only on `ratatui-core`.
//!
//! # Render in a Ratatui application
//!
//! Prepare a [`DiffDocument`] once and keep a [`DiffState`] between frames to retain the viewport
//! and cached layout. Build a [`Diff`] in the draw closure to select presentation options:
//!
//! ```
//! use ratatui::Terminal;
//! use ratatui::backend::TestBackend;
//! use ratatui_diff::{Diff, DiffDocument, DiffState, ViewMode};
//!
//! let document = DiffDocument::from_text("hello world\n", "hello Rust\n");
//! let mut state = DiffState::new();
//! let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
//! terminal.draw(|frame| {
//!     let diff = Diff::new(&document).mode(ViewMode::Split);
//!     frame.render_stateful_widget(&diff, frame.area(), &mut state);
//! })?;
//! // Call from your PageDown handler, then redraw.
//! state.scroll_pages(1);
//! # Ok::<(), std::convert::Infallible>(())
//! ```
//!
//! This example uses an in-memory terminal. The draw closure also works with Crossterm or Termion.
//! The [interactive viewer] includes key bindings, an event loop, and terminal restoration.
//!
//! [interactive viewer]: https://github.com/joshka/ratatui-diff/blob/main/examples/viewer.rs
//!
//! # Choose an input
//!
//! - [`DiffDocument::from_text`] compares old/new UTF-8 source with three context lines.
//! - [`DiffDocument::compare`] selects the number of context lines when generating hunks.
//! - [`DiffDocument::parse`] reads unified or multi-file Git patches; errors include byte offsets
//!   when available.
//! - [`DiffDocument::new`] validates caller-provided files, hunks, lines, and highlight ranges.
//!
//! ```
//! use ratatui_diff::DiffDocument;
//!
//! let patch = "--- a/greeting\n+++ b/greeting\n@@ -1 +1 @@\n-hello world\n+hello Rust\n";
//! let document = DiffDocument::parse(patch)?;
//! assert_eq!(document.files().len(), 1);
//! # Ok::<(), ratatui_diff::DiffError>(())
//! ```
//!
//! A patch displays only its supplied changes and context. Binary changes appear as
//! summaries. Combined merge diffs are rejected. Source content retains CRLF and final-newline
//! differences; control characters are displayed as visible text.
//!
//! # Configure appearance
//!
//! The defaults are unified view, visible line numbers and word highlights, no wrapping or
//! whitespace markers, four-cell tab stops, and the dark theme. Split replacements pair lines in
//! source order and pad the shorter side. Each [`DiffTheme`] style can be customized independently.
//!
//! ```
//! use ratatui_diff::{Diff, DiffDocument, DiffTheme, ViewMode};
//!
//! let document = DiffDocument::from_text("old\n", "new\n");
//! let diff = Diff::new(&document)
//!     .mode(ViewMode::Split)
//!     .theme(DiffTheme::light())
//!     .wrap(true)
//!     .whitespace(true)
//!     .tab_width(4);
//! ```
//!
//! Wrapped split rows use the taller side's height. Without wrapping, horizontal scrolling is
//! synchronized across panes. Graphemes are never split at viewport edges. Disable inline
//! highlights with [`Diff::word_highlights`] when whole-line styling is sufficient.
//!
//! # Navigate and retain the viewport
//!
//! Render once before page navigation or source mapping so [`DiffState`] has the viewport
//! dimensions and row positions. Scrolling counts displayed rows, including headers and wrapped
//! continuations. Source positions identify a file, an old or new side, and a one-based line
//! number.
//!
//! - Lines, pages, half pages: [`scroll_lines`](DiffState::scroll_lines),
//!   [`scroll_pages`](DiffState::scroll_pages),
//!   [`scroll_half_pages`](DiffState::scroll_half_pages).
//! - Horizontal cells: [`scroll_horizontal`](DiffState::scroll_horizontal).
//! - Start/end: [`start`](DiffState::start), [`end`](DiffState::end).
//! - Hunk/file boundaries: [`next_hunk`](DiffState::next_hunk),
//!   [`previous_hunk`](DiffState::previous_hunk), [`next_file`](DiffState::next_file),
//!   [`previous_file`](DiffState::previous_file).
//! - Source positions: [`scroll_to_source`](DiffState::scroll_to_source),
//!   [`source_at`](DiffState::source_at).
//!
//! ```
//! use ratatui_core::buffer::Buffer;
//! use ratatui_core::layout::Rect;
//! use ratatui_core::widgets::StatefulWidget;
//! use ratatui_diff::{Diff, DiffDocument, DiffState, Side, SourcePosition};
//!
//! let document = DiffDocument::from_text("before\n", "after\n");
//! let area = Rect::new(0, 0, 80, 3);
//! let mut buffer = Buffer::empty(area);
//! let mut state = DiffState::new();
//! (&Diff::new(&document)).render(area, &mut buffer, &mut state);
//! assert!(state.scroll_to_source(SourcePosition {
//!     file: 0,
//!     side: Side::New,
//!     line: 1
//! }));
//! ```
//!
//! Resizing or switching modes retains the nearest available source anchor. Replacing the document
//! resets navigation on the next render. Theme and word-highlight changes preserve layout. Give
//! each independently navigated pane its own state, even when both borrow the same document.
//!
//! # Select source for copying
//!
//! Keep anchor and focus in [`DiffState::set_selection`], then extend focus with
//! [`SelectionMotion`] from host key bindings. Pointer handlers can convert a hit [`SourceRange`]
//! with [`start_boundary`](SourceRange::start_boundary) and
//! [`end_boundary`](SourceRange::end_boundary). Selection stays within one file and side, survives
//! resizing and view changes, and clears on document replacement.
//!
//! [`selected_text`](DiffState::selected_text) returns original source bytes for the host's
//! clipboard integration. It preserves tabs, controls, CRLF, and final-newline state; it excludes
//! gutters, wrapping, padding, and display notation. Invalid grapheme boundaries and missing patch
//! context are rejected. [`Diff::selection_style`] controls the overlay applied after word
//! highlights.
//!
//! # Search available source
//!
//! Update a case-sensitive literal query in your event handler, then navigate cached occurrences.
//! Both-side search counts shared context once. Restrict search with `Some(Side::Old)` or
//! `Some(Side::New)`; an empty query clears it. Matches follow document order and navigation wraps.
//! Omitted patch context, file headers, and metadata are not searched.
//!
//! ```
//! use ratatui_diff::{DiffDocument, DiffState};
//!
//! let document = DiffDocument::from_text("timeout: 500\n", "timeout: 1500\n");
//! let mut state = DiffState::new();
//! state.set_search(&document, "timeout", None);
//! assert_eq!(state.search_matches().len(), 2);
//! assert!(state.next_match());
//! assert_eq!(state.active_match(), Some(0));
//! // Render the widget to reveal the selected occurrence.
//! assert!(state.previous_match());
//! assert_eq!(state.active_match(), Some(1));
//! ```
//!
//! Matches retain source byte ranges through wrapping, resizing, and mode changes. Search styles
//! compose over changed-word styles; the selected occurrence uses reverse video in every preset,
//! including monochrome. Query updates scan available source synchronously; frame rendering reuses
//! the results. Updating a query or side clears the active match, while identical updates preserve
//! it. Rendering a replacement document clears search; the host can then reapply its query.
//!
//! # Large documents
//!
//! Comparison is synchronous; prepare large documents outside the event loop. Rendering reuses the
//! comparison results and draws only visible rows, but first layout and resize can process the
//! whole document. Large replacements fall back to whole-line styling when they exceed the
//! word-comparison size limits.
//!
//! # Compatibility
//!
//! This pre-1.0 API may change between minor releases.

mod compare;
mod model;
mod parse;
mod search;
mod selection;
mod theme;
mod widget;

pub use model::{DiffDocument, DiffError, DiffFile, DiffLine, Hunk, LineKind, Side};
pub use selection::{SelectionMotion, SourceBoundary, SourceSelection};
pub use theme::DiffTheme;
pub use widget::{Diff, DiffState, HitTest, SourcePosition, SourceRange, ViewMode};
