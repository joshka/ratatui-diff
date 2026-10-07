//! A navigable unified and split diff widget for Ratatui.
//!
//! Prepare a [`DiffDocument`] once, borrow it through [`Diff`], and keep [`DiffState`]
//! between frames. The library owns comparison, patch parsing, layout, and navigation;
//! your application owns events, terminal setup, files, and repository access.
//!
//! ```
//! use ratatui_core::buffer::Buffer;
//! use ratatui_core::layout::Rect;
//! use ratatui_core::widgets::StatefulWidget;
//! use ratatui_diff::{Diff, DiffDocument, DiffState, ViewMode};
//!
//! let document = DiffDocument::from_text("hello world\n", "hello Rust\n");
//! let widget = Diff::new(&document).mode(ViewMode::Split);
//! let area = Rect::new(0, 0, 80, 20);
//! let mut buffer = Buffer::empty(area);
//! let mut state = DiffState::new();
//! (&widget).render(area, &mut buffer, &mut state);
//! state.scroll_pages(1);
//! ```
//!
//! Input is UTF-8. Unified patches include only their available context; navigation
//! cannot recover omitted source lines. Combined merge diffs are unsupported.
//! Comparison and first layout are synchronous; prepare large documents outside the
//! event loop. Steady frames draw indexed visible rows without redoing comparison.
//! This pre-1.0 API may change between minor releases.

mod compare;
mod model;
mod parse;
mod theme;
mod widget;

pub use model::{DiffDocument, DiffError, DiffFile, DiffLine, Hunk, LineKind, Side};
pub use theme::DiffTheme;
pub use widget::{Diff, DiffState, SourcePosition, ViewMode};
