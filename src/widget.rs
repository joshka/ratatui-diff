//! Prepared source rows, width-dependent screen rows, and viewport navigation.
//!
//! [`Diff`] borrows immutable content; [`DiffState`] owns its cached geometry and offsets.
//! Preparation may traverse the document. Drawing uses the indexed visible rows.

use std::collections::HashSet;
use std::ops::Range;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::search::Search;
use crate::{DiffDocument, DiffLine, DiffTheme, LineKind, Side};

/// Diff presentation mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Original and modified lines in unified order, with both line-number columns. The default.
    #[default]
    Unified,

    /// Original source on the left and modified source on the right.
    ///
    /// Replacement lines use bounded similarity pairing; unmatched lines leave a blank opposite
    /// pane. Unanchored gaps and oversized runs pair in source order. Wrapped pairs use the taller
    /// side's height, and horizontal scrolling moves both panes.
    Split,
}

/// A numbered source location within a document.
///
/// Used by [`DiffState::source_at`] and [`DiffState::scroll_to_source`]. This value is not
/// validated on construction; a location can refer to a missing file or omitted patch context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// File index, starting at zero.
    pub file: usize,

    /// Original or modified source.
    pub side: Side,

    /// Source line number, starting at one.
    pub line: usize,
}

/// A half-open UTF-8 byte range within a source line, excluding its line ending.
///
/// Retain only with the document that produced it. Rendering never reconstructs omitted context.
/// Hit-testing returns whole graphemes; literal search may match part of a grapheme, and rendering
/// emphasizes every intersecting grapheme.
/// Public field construction is unchecked; hit-testing returns valid grapheme ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    /// File, side, and one-based line identity.
    pub position: SourcePosition,

    /// Source bytes; expanded tabs and escapes share their original grapheme range.
    pub bytes: Range<usize>,
}

/// A retained context range that can be expanded without fetching source.
///
/// Ranges are one-based and end-exclusive. Handles belong to the document and context radius
/// that produced them; state rejects stale handles. Both ranges contain the same number of lines.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextFold {
    /// Zero-based file index.
    pub file: usize,
    /// Zero-based hunk index.
    pub hunk: usize,
    /// Hidden original source lines.
    pub old: Range<usize>,
    /// Hidden modified source lines.
    pub new: Range<usize>,
    document: u64,
    radius: usize,
}

impl ContextFold {
    /// Exact number of retained unchanged lines represented by this fold.
    pub fn line_count(&self) -> usize {
        self.old.len()
    }

    fn contains(&self, position: SourcePosition) -> bool {
        self.file == position.file
            && match position.side {
                Side::Old => self.old.contains(&position.line),
                Side::New => self.new.contains(&position.line),
            }
    }
}

/// A validated file identity used by whole-file expansion controls.
///
/// Obtain a handle from [`DiffState::file_folds`] after rendering, or from
/// [`DiffDocument::file_fold`] to restore expansion before the first frame. Handles retain their
/// document identity and remain valid through presentation changes and document clones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileFold {
    pub(crate) file: usize,
    pub(crate) document: u64,
}

impl FileFold {
    /// Zero-based index of the file within its document.
    pub fn file(&self) -> usize {
        self.file
    }
}

/// Cached file labels and supplied-line change counts; never recomputed in a steady frame.
#[derive(Debug)]
struct FileSummary {
    path: String,
    changes: String,
    added: usize,
    removed: usize,
}

impl FileSummary {
    fn new(file: &crate::DiffFile) -> Self {
        let path = match (&file.old_path, &file.new_path) {
            (Some(old), Some(new)) if old != new => format!("{old} → {new}"),
            (_, Some(path)) | (Some(path), _) => path.clone(),
            (None, None) => unreachable!("validated files have a path"),
        };
        let mut added = 0;
        let mut removed = 0;
        for line in file.hunks.iter().flat_map(|hunk| &hunk.lines) {
            match line.kind {
                LineKind::Insert => added += 1,
                LineKind::Delete => removed += 1,
                LineKind::Context => {}
            }
        }
        let changes = if file.binary {
            "binary".into()
        } else if file.hunks.is_empty() && !file.metadata.is_empty() {
            "metadata".into()
        } else {
            format!("+{added} −{removed}")
        };
        Self {
            path,
            changes,
            added,
            removed,
        }
    }

    fn label(&self, expanded: bool, width: usize) -> String {
        let disclosure = if expanded { '▾' } else { '▸' };
        let suffix = format!(" · {}", self.changes);
        let path_width = width.saturating_sub(2 + suffix.width());
        if path_width == 0 {
            return format!("{disclosure} {}", self.changes);
        }
        let path = if self.path.width() <= path_width {
            self.path.clone()
        } else {
            let mut path = String::new();
            let mut used = 0;
            for grapheme in self.path.graphemes(true) {
                let cells = grapheme.width();
                if used + cells > path_width.saturating_sub(1) {
                    break;
                }
                path.push_str(grapheme);
                used += cells;
            }
            path.push('…');
            path
        };
        format!("{disclosure} {path}{suffix}")
    }
}

/// A navigation request resolved after rebuilding the displayed-row index.
#[derive(Debug, Clone, Copy)]
enum RevealTarget {
    Source(SourcePosition),
    File(usize),
    Hunk { file: usize, hunk: usize },
}

/// The semantic region occupying a cell in the last rendered diff viewport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTest {
    /// A whole-file expansion control. Metadata, hunk headers, and gaps remain [`Self::Header`].
    FileHeader {
        /// Document-scoped file identity; inspect expansion with [`DiffState::file_expanded`].
        fold: FileFold,
    },
    /// Collapsed retained context. This is an expansion control, without source bytes.
    Fold {
        /// Stable source ranges represented by the summary.
        fold: ContextFold,
    },
    /// Source text, with both sides available for unified context.
    Source {
        /// Original source range, when available.
        old: Option<SourceRange>,
        /// Modified source range, when available.
        new: Option<SourceRange>,
    },
    /// A line-number/change-marker gutter, without source bytes.
    Gutter {
        /// Original source line, when available.
        old: Option<SourcePosition>,
        /// Modified source line, when available.
        new: Option<SourcePosition>,
    },
    /// Metadata, a hunk header, or an unavailable-context cue.
    /// Unavailable context has no expansion control or source coordinates.
    Header {
        /// Zero-based file index.
        file: usize,
    },
    /// Presentation-only missing-final-newline notation.
    FinalNewline {
        /// Original source line, when available.
        old: Option<SourcePosition>,
        /// Modified source line, when available.
        new: Option<SourcePosition>,
    },
    /// The separator between split panes.
    Separator,
    /// Blank cells, including clipped glyphs and exhausted split continuations.
    Padding,
}

/// A display unit after tab/control expansion, positioned in terminal cells.
///
/// Ordinary units contain one grapheme; tabs become individual spaces and a control escape stays
/// one unit. Clipping and wrapping never divide a unit. Highlight overlap is resolved from source
/// bytes during preparation, before transformations change the displayed text.
#[derive(Debug)]
struct Glyph {
    text: Range<usize>,
    // Synthetic final-newline notation has no source byte range.
    bytes: Option<Range<usize>>,

    /// Cell offset from the start of the unwrapped content, excluding the gutter.
    column: usize,

    width: usize,
    emphasized: bool,

    /// A generated space or tab marker, rather than a literal source dot or arrow.
    whitespace_marker: bool,
}

/// One source line or synthetic header, with width-independent display geometry.
///
/// Source numbers survive wrapping. Missing split partners use empty context content with neither
/// number, so they cannot be returned by source lookup.
#[derive(Debug)]
struct Content {
    // One buffer owns the source copy and appended transformations. Ordinary glyphs index
    // its source prefix; source byte ranges remain independent of display transformations.
    text: String,
    glyphs: Box<[Glyph]>,
    kind: LineKind,
    old: Option<usize>,
    new: Option<usize>,
    width: usize,
}

/// A logical row before wrapping: one unified line, a split pair, or a full-width header.
///
/// `right` is present for split content, including an empty partner. Headers have no source
/// numbers; their file/hunk IDs identify navigation boundaries.
#[derive(Debug)]
struct Row {
    file: usize,
    hunk: Option<usize>,
    left: Content,
    right: Option<Content>,
    header: bool,
    fold: Option<usize>,
    hidden: Option<usize>,
    gap: bool,
    file_header: bool,
}

/// One displayed continuation of a logical row.
///
/// `row` indexes `DiffState::rows`; `left` and `right` index glyphs, not bytes or cells. Empty
/// ranges pad the shorter wrapped side while retaining the logical row's source mapping.
#[derive(Debug)]
struct ScreenRow {
    row: usize,
    left: Range<usize>,
    right: Range<usize>,

    /// True after the first screen row, even when one source side is empty.
    continuation: bool,
}

/// Inputs that invalidate screen geometry.
///
/// Height only changes clamping. Theme and highlight visibility affect painting, so neither belongs
/// in this key. Some changes rebuild only screen rows; `prepare` decides whether glyphs are
/// reusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayoutKey {
    document: u64,
    width: u16,
    mode: ViewMode,
    numbers: bool,
    wrap: bool,
    whitespace: bool,
    tab: usize,
    context: Option<usize>,
}

/// Viewport state and prepared layout cache.
///
/// Render once before page navigation or source mapping. Reuse this state between frames;
/// replacing it discards layout caches. Offsets use `usize`, independent of terminal height limits.
///
/// Line/page commands operate on displayed rows, including headers and wrapped continuations.
/// [`source_at`](Self::source_at) and [`scroll_to_source`](Self::scroll_to_source) use source
/// line numbers instead. Page commands use the last rendered height. Navigation clamps to the
/// final full viewport; a requested source line near the end may appear below the first row.
/// Mode/width changes preserve a source anchor, while document replacement resets offsets.
/// Keep one state per independently navigated view.
///
/// # Navigation
///
/// - Inspect the last render with [`offset`](Self::offset),
///   [`horizontal_offset`](Self::horizontal_offset), and [`row_count`](Self::row_count).
/// - Move relatively with [`scroll_lines`](Self::scroll_lines),
///   [`scroll_pages`](Self::scroll_pages), [`scroll_half_pages`](Self::scroll_half_pages), and
///   [`scroll_horizontal`](Self::scroll_horizontal).
/// - Jump with [`start`](Self::start), [`end`](Self::end), or the file/hunk navigation methods.
/// - Map source positions with [`source_at`](Self::source_at) and
///   [`scroll_to_source`](Self::scroll_to_source).
///
/// Navigation uses the last rendered layout. Changing widget options alone does not refresh it.
/// Before the first render, row counts and offsets are zero and navigation has no effect.
///
/// # Example
///
/// ```
/// use ratatui_core::buffer::Buffer;
/// use ratatui_core::layout::Rect;
/// use ratatui_core::widgets::StatefulWidget;
/// use ratatui_diff::{Diff, DiffDocument, DiffState, Side};
///
/// let document = DiffDocument::from_text("old\n", "new\n");
/// let area = Rect::new(0, 0, 40, 2);
/// let mut buffer = Buffer::empty(area);
/// let mut state = DiffState::new();
/// (&Diff::new(&document)).render(area, &mut buffer, &mut state);
/// state.end();
/// let source = state.source_at(state.offset(), Side::Old).unwrap();
/// assert_eq!(source.line, 1);
/// assert!(state.scroll_to_source(source));
/// ```
#[derive(Debug, Default)]
pub struct DiffState {
    // Last painted rectangle and offsets; navigation must redraw before cell lookup.
    rendered: Option<(Rect, usize, usize)>,
    selection: Option<crate::SourceSelection>,
    selection_document: Option<u64>,
    reveal_selection: bool,
    offset: usize,
    horizontal: usize,
    height: usize,
    key: Option<LayoutKey>,
    rows: Vec<Row>,
    screen: Vec<ScreenRow>,
    // Absolute screen-row indexes of file and hunk headers, in display order.
    files: Vec<usize>,
    // (first displayed row, file, logical hunk-header row), including closed files.
    hunks: Vec<(usize, usize, usize)>,
    // Sorted (file index, source line, first screen row) tuples for binary lookup.
    old_sources: Vec<(usize, usize, usize)>,
    new_sources: Vec<(usize, usize, usize)>,
    digits: usize,
    max_width: usize,
    content_width: usize,
    search: Search,
    folds: Vec<ContextFold>,
    expanded: HashSet<ContextFold>,
    folds_dirty: bool,
    pending: Option<RevealTarget>,
    file_document: Option<u64>,
    file_folds: Vec<FileFold>,
    file_summaries: Vec<FileSummary>,
    // Document-wide supplied-line counts, prepared once with file summaries.
    statistics: String,
    collapsed_files: HashSet<usize>,
}

impl DiffState {
    /// All supplied files from the last render, including binary and metadata-only files.
    ///
    /// Before the first render this is empty; [`DiffDocument::file_fold`] issues validated handles
    /// for restoring saved expansion preferences before drawing.
    pub fn file_folds(&self) -> &[FileFold] {
        &self.file_folds
    }

    /// Whether a file is expanded. Files default to expanded.
    ///
    /// An unbound state accepts document-issued handles without binding itself. After the first
    /// setter or render, handles from another document return false.
    pub fn file_expanded(&self, fold: &FileFold) -> bool {
        self.file_document
            .is_none_or(|document| document == fold.document)
            && !self.collapsed_files.contains(&fold.file)
    }

    /// Expand or collapse a whole file on the next render, retaining its header.
    ///
    /// A fresh state binds to the first handle's document, allowing initial collapse without an
    /// expanded first frame. Further foreign handles return false until a render rebinds state.
    /// Rendering another document clears file preferences; clones retain their identities.
    /// Valid unchanged requests return true and leave hit-testing intact. A real change invalidates
    /// mapping until redraw. Collapsing the file at the viewport anchors its retained header;
    /// off-screen changes preserve the current source anchor. Nested context expansion is retained.
    /// Manual collapse cancels pending search and selection reveal so the file stays closed.
    pub fn set_file_expanded(&mut self, fold: &FileFold, expanded: bool) -> bool {
        if self
            .file_document
            .is_some_and(|document| document != fold.document)
        {
            return false;
        }
        self.file_document = Some(fold.document);
        if self.collapsed_files.contains(&fold.file) == !expanded {
            return true;
        }
        if expanded {
            self.collapsed_files.remove(&fold.file);
        } else {
            self.collapsed_files.insert(fold.file);
        }
        let current = self
            .screen
            .get(self.offset)
            .is_some_and(|screen| self.rows[screen.row].file == fold.file);
        if !expanded {
            self.pending = self.pending.filter(|target| match target {
                RevealTarget::Source(position) => position.file != fold.file,
                RevealTarget::Hunk { file, .. } => *file != fold.file,
                RevealTarget::File(_) => true,
            });
        }
        if current {
            self.pending = Some(RevealTarget::File(fold.file));
        }
        self.folds_dirty = true;
        self.rendered = None;
        self.search.reveal = false;
        self.reveal_selection = false;
        true
    }

    /// All retained-context fold candidates from the last render, including expanded ones.
    pub fn context_folds(&self) -> &[ContextFold] {
        &self.folds
    }

    /// Whether a current fold is expanded. Unknown or stale handles return false.
    pub fn context_expanded(&self, fold: &ContextFold) -> bool {
        self.expanded.contains(fold) && self.current_fold(fold)
    }

    /// Expand or collapse retained context on the next render.
    ///
    /// Returns false for unknown or stale handles. An unchanged valid request returns true.
    /// Changing expansion invalidates hit-testing until redraw. A visible expanded summary is
    /// replaced at the viewport by its first revealed line; off-screen changes preserve the anchor.
    /// Manual collapse does not reveal an already active search result again.
    pub fn set_context_expanded(&mut self, fold: &ContextFold, expanded: bool) -> bool {
        if !self.current_fold(fold) {
            return false;
        }
        if self.expanded.contains(fold) != expanded {
            if expanded {
                self.expanded.insert(fold.clone());
                // If the control is visible, replace it with the first revealed source line.
                // Off-screen programmatic expansion keeps the current viewport anchor.
                let visible =
                    self.screen
                        .iter()
                        .skip(self.offset)
                        .take(self.height)
                        .any(|screen| {
                            self.rows[screen.row]
                                .fold
                                .is_some_and(|index| self.folds[index] == *fold)
                        });
                if visible {
                    self.pending = Some(RevealTarget::Source(SourcePosition {
                        file: fold.file,
                        side: Side::Old,
                        line: fold.old.start,
                    }));
                }
            } else {
                self.expanded.remove(fold);
            }
            self.folds_dirty = true;
            self.rendered = None;
            self.search.reveal = false;
        }
        true
    }

    fn current_fold(&self, fold: &ContextFold) -> bool {
        let index = self.folds.partition_point(|candidate| {
            (candidate.file, candidate.old.start) < (fold.file, fold.old.start)
        });
        self.folds.get(index) == Some(fold)
    }

    fn expand_source(&mut self, position: SourcePosition) -> bool {
        let sources = match position.side {
            Side::Old => &self.old_sources,
            Side::New => &self.new_sources,
        };
        if sources
            .binary_search_by_key(&(position.file, position.line), |&(file, line, _)| {
                (file, line)
            })
            .is_err()
        {
            return false;
        }
        let mut changed = false;
        if self.collapsed_files.contains(&position.file) {
            let fold = self.file_folds[position.file];
            let selection_reveal = self.reveal_selection;
            self.set_file_expanded(&fold, true);
            self.reveal_selection = selection_reveal;
            changed = true;
        }
        let index = self.folds.partition_point(|fold| {
            let range = match position.side {
                Side::Old => &fold.old,
                Side::New => &fold.new,
            };
            fold.file < position.file || (fold.file == position.file && range.end <= position.line)
        });
        if let Some(fold) = self
            .folds
            .get(index)
            .filter(|fold| fold.contains(position))
            .cloned()
            && !self.context_expanded(&fold)
        {
            self.set_context_expanded(&fold, true);
            changed = true;
        }
        changed
    }

    /// Inspect the current source selection. It survives layout changes, but not document
    /// replacement.
    pub fn selection(&self) -> Option<crate::SourceSelection> {
        self.selection
    }

    /// Set anchor/focus from host keyboard or pointer input.
    ///
    /// Rejects invalid source boundaries, cross-file/side ranges, and omitted patch context.
    /// Invalid input leaves the current selection unchanged. Validation borrows source text;
    /// copying occurs only when extracting it. The selection belongs to `document`; rendering
    /// another document clears it.
    pub fn set_selection(
        &mut self,
        document: &DiffDocument,
        selection: crate::SourceSelection,
    ) -> bool {
        if !selection.is_valid(document) {
            return false;
        }
        self.selection = Some(selection);
        self.selection_document = Some(document.id);
        true
    }

    /// Remove the selection without changing navigation.
    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_document = None;
        self.reveal_selection = false;
    }

    /// Extend focus using source graphemes and line boundaries; anchor stays fixed.
    ///
    /// Returns false at a source boundary or missing patch context. Hosts bind their own keys.
    pub fn extend_selection(
        &mut self,
        document: &DiffDocument,
        motion: crate::SelectionMotion,
    ) -> bool {
        if self.selection_document != Some(document.id) {
            return false;
        }
        let Some(mut selection) = self.selection else {
            return false;
        };
        let Some(focus) = document.move_boundary(selection.focus, motion) else {
            return false;
        };
        selection.focus = focus;
        self.set_selection(document, selection)
    }

    /// Reveal the selection focus on the next render, without changing its source boundaries.
    ///
    /// Returns `false` when no selection exists. Reveals the grapheme after the boundary, or the
    /// final source grapheme at end of line, using the next frame's wrapping and pane geometry.
    /// Moves only enough to include that grapheme, clamping at the final viewport. Empty lines
    /// reveal their source row; zero-width content cannot display a glyph. End/LF uses the final
    /// source grapheme. Successful requests invalidate hit-testing until the next render.
    /// Takes precedence over a pending search reveal in that frame. Subsequent manual scrolling
    /// remains under host control; layout changes do not automatically reveal selection focus.
    pub fn reveal_selection(&mut self) -> bool {
        self.reveal_selection = self.selection.is_some();
        if self.reveal_selection {
            self.rendered = None;
        }
        self.reveal_selection
    }

    /// Obtain exact selected text for host-controlled copying, without clipboard I/O.
    pub fn selected_text(&self, document: &DiffDocument) -> Option<String> {
        if self.selection_document != Some(document.id) {
            return None;
        }
        self.selection?.text(document)
    }

    /// Construct an empty viewport with no prepared layout. Equivalent to `Default::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update case-sensitive literal search over available source lines, outside rendering.
    ///
    /// `None` searches both sides, counting shared context once; `Some` restricts the side.
    /// Results follow file, hunk, supplied line, then byte order. Occurrences do not overlap or
    /// span lines. Headers, metadata, omitted patch context, and synthetic cues are not searched.
    /// Empty queries clear results. A changed query, side, or document clears the active match;
    /// identical updates reuse results. Select a result with [`next_match`](Self::next_match).
    /// Rendering a different document clears the search rather than scanning during a frame.
    pub fn set_search(&mut self, document: &DiffDocument, query: &str, side: Option<Side>) {
        self.search.update(document, query, side);
    }

    /// Current literal query. Empty after document replacement or
    /// [`clear_search`](Self::clear_search).
    pub fn search_query(&self) -> &str {
        &self.search.query
    }

    /// Cached source occurrences in document order, independent of width and presentation mode.
    ///
    /// In a both-side search, shared context uses the new side's identity and paints both panes.
    pub fn search_matches(&self) -> &[SourceRange] {
        &self.search.matches
    }

    /// Selected occurrence's zero-based index, or `None` before navigation or when empty.
    pub fn active_match(&self) -> Option<usize> {
        self.search.active
    }

    /// Clear the query, cached results, and active match without changing the viewport.
    pub fn clear_search(&mut self) {
        self.search = Search::default();
    }

    /// Select the next occurrence, wrapping from the last to the first.
    ///
    /// With no active match, selects the first. Returns `false` when empty. The next render reveals
    /// the match's first intersecting grapheme, including wrapped or horizontally off-screen text.
    /// Resizing and mode changes retain the source occurrence and reveal it in the new layout.
    pub fn next_match(&mut self) -> bool {
        self.navigate_match(true)
    }

    /// Select the previous occurrence, wrapping from the first to the last.
    ///
    /// With no active match, selects the last. Otherwise behaves like
    /// [`next_match`](Self::next_match).
    pub fn previous_match(&mut self) -> bool {
        self.navigate_match(false)
    }

    fn navigate_match(&mut self, forward: bool) -> bool {
        let count = self.search.matches.len();
        if count == 0 {
            return false;
        }
        self.search.active = Some(match (self.search.active, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(n), true) => (n + 1) % count,
            (Some(n), false) => (n + count - 1) % count,
        });
        self.search.reveal = true;
        self.rendered = None;
        true
    }

    fn reveal_match(&mut self) {
        if !self.search.reveal {
            return;
        }
        self.search.reveal = false;
        let Some(found) = self.search.active.map(|n| &self.search.matches[n]) else {
            return;
        };
        let position = found.position;
        let byte = found.bytes.start;
        self.reveal_boundary(position, byte, false);
    }

    fn reveal_boundary(&mut self, position: SourcePosition, byte: usize, selection: bool) {
        let sources = match position.side {
            Side::Old => &self.old_sources,
            Side::New => &self.new_sources,
        };
        let Ok(n) =
            sources.binary_search_by_key(&(position.file, position.line), |&(f, l, _)| (f, l))
        else {
            return;
        };
        let first = sources[n].2;
        let row_index = self.screen[first].row;
        let row = &self.rows[row_index];
        let right =
            self.key.is_some_and(|k| k.mode == ViewMode::Split) && position.side == Side::New;
        let content = if right {
            row.right.as_ref().unwrap_or(&row.left)
        } else {
            &row.left
        };
        let glyph = content
            .glyphs
            .partition_point(|g| g.bytes.as_ref().is_some_and(|bytes| bytes.end <= byte));
        // End/LF boundaries reveal the final source glyph, never a synthetic newline cue.
        let glyph = if content.glyphs.get(glyph).is_some_and(|g| g.bytes.is_some()) {
            Some(glyph)
        } else {
            content.glyphs.iter().rposition(|g| g.bytes.is_some())
        };
        let g = glyph.and_then(|n| content.glyphs.get(n));
        let wrapped = self.key.is_some_and(|k| k.wrap);
        let target = if let Some(glyph) = glyph
            && wrapped
        {
            self.screen.partition_point(|s| {
                s.row < row_index
                    || (s.row == row_index
                        && if right {
                            s.right.end <= glyph
                        } else {
                            s.left.end <= glyph
                        })
            })
        } else {
            first
        };
        if target < self.offset || target >= self.offset.saturating_add(self.height) {
            self.offset = if !selection || target < self.offset {
                target
            } else {
                target.saturating_sub(self.height.saturating_sub(1))
            };
        }
        if let Some(g) = g
            && !wrapped
            && (g.column < self.horizontal
                || g.column + g.width > self.horizontal + self.content_width)
        {
            let horizontal = if !selection || g.column < self.horizontal {
                g.column
            } else {
                (g.column + g.width).saturating_sub(self.content_width)
            };
            self.horizontal = horizontal.min(self.max_width.saturating_sub(self.content_width));
        }
        self.clamp();
    }

    /// Zero-based absolute index of the top displayed row in the last rendered layout.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Horizontal cell offset, shared between split panes.
    pub fn horizontal_offset(&self) -> usize {
        self.horizontal
    }

    /// Total displayed rows in the last rendered layout, including headers and continuations.
    pub fn row_count(&self) -> usize {
        self.screen.len()
    }

    /// Discard cell mapping before a host changes geometry, options, or documents.
    ///
    /// The next render refreshes it. Source/navigation layout remains cached.
    pub fn invalidate_hit_testing(&mut self) {
        self.rendered = None;
    }

    /// Inspect an absolute terminal cell in the most recent rendered viewport.
    ///
    /// Returns `None` outside that viewport, before rendering, or after navigation until redraw.
    /// Widget changes take effect at rendering; invalidate explicitly if events arrive first.
    /// Source ranges use line-local UTF-8 bytes and include whole graphemes, tabs, and escapes.
    /// Blank clipping cells never claim source bytes. Unified context returns both sides.
    ///
    /// ```
    /// use ratatui_core::buffer::Buffer;
    /// use ratatui_core::layout::Rect;
    /// use ratatui_core::widgets::StatefulWidget;
    /// use ratatui_diff::{Diff, DiffDocument, DiffState, HitTest};
    ///
    /// let document = DiffDocument::from_text("", "界\n");
    /// let area = Rect::new(10, 5, 20, 4);
    /// let mut state = DiffState::new();
    /// let widget = Diff::new(&document).line_numbers(false);
    /// (&widget).render(area, &mut Buffer::empty(area), &mut state);
    /// let Some(HitTest::Source {
    ///     new: Some(source), ..
    /// }) = state.hit_test(12, 7)
    /// else {
    ///     panic!("the second cell of 界 belongs to the same source grapheme");
    /// };
    /// assert_eq!(source.bytes, 0..3);
    /// assert_eq!(source.position.line, 1);
    /// ```
    pub fn hit_test(&self, x: u16, y: u16) -> Option<HitTest> {
        let (area, offset, horizontal) = self.rendered?;
        if x < area.x
            || x >= area.right()
            || y < area.y
            || y >= area.bottom()
            || offset != self.offset
            || horizontal != self.horizontal
        {
            return None;
        }
        let Some(screen) = self.screen.get(offset + usize::from(y - area.y)) else {
            return Some(HitTest::Padding);
        };
        let row = &self.rows[screen.row];
        if row.file_header {
            return Some(HitTest::FileHeader {
                fold: self.file_folds[row.file],
            });
        }
        if let Some(fold) = row.fold {
            return Some(HitTest::Fold {
                fold: self.folds[fold].clone(),
            });
        }
        if row.header {
            return Some(HitTest::Header { file: row.file });
        }
        let key = self.key?;
        let split = key.mode == ViewMode::Split;
        let pane = if split {
            usize::from(area.width.saturating_sub(1)) / 2
        } else {
            usize::from(area.width)
        };
        let mut column = usize::from(x - area.x);
        if split && column == pane {
            return Some(HitTest::Separator);
        }
        let right = split && column > pane;
        if right {
            column -= pane + 1;
        }
        if column >= pane {
            return Some(HitTest::Padding);
        }
        let content = if right {
            row.right.as_ref()?
        } else {
            &row.left
        };
        let segment = if right { &screen.right } else { &screen.left };
        let position = |side, line: Option<usize>| {
            line.map(|line| SourcePosition {
                file: row.file,
                side,
                line,
            })
        };
        let old = if right {
            None
        } else {
            position(Side::Old, content.old)
        };
        let new = if split && !right {
            None
        } else {
            position(Side::New, content.new)
        };
        let gutter = if !key.numbers {
            1
        } else if split {
            self.digits + 2
        } else {
            self.digits * 2 + 3
        };
        let gutter = gutter.min(pane);
        if screen.continuation && segment.is_empty() || old.is_none() && new.is_none() {
            return Some(HitTest::Padding);
        }
        if column < gutter {
            return Some(HitTest::Gutter { old, new });
        }
        let width = pane - gutter;
        let glyphs = &content.glyphs[segment.clone()];
        let base = if key.wrap {
            glyphs.first().map_or(0, |g| g.column)
        } else {
            horizontal
        };
        let target = base + column - gutter;
        let index = glyphs.partition_point(|g| g.column + g.width <= target);
        let Some(glyph) = glyphs.get(index) else {
            return Some(HitTest::Padding);
        };
        if glyph.column < base || glyph.column > target || glyph.column + glyph.width > base + width
        {
            return Some(HitTest::Padding);
        }
        Some(match &glyph.bytes {
            Some(bytes) => HitTest::Source {
                old: old.map(|position| SourceRange {
                    position,
                    bytes: bytes.clone(),
                }),
                new: new.map(|position| SourceRange {
                    position,
                    bytes: bytes.clone(),
                }),
            },
            None => HitTest::FinalNewline { old, new },
        })
    }

    /// Move by displayed rows; negative values move toward the start.
    ///
    /// Clamps to the first or final full viewport. Horizontal position is unchanged.
    pub fn scroll_lines(&mut self, lines: isize) {
        self.offset = self.offset.saturating_add_signed(lines);
        self.clamp();
    }

    /// Move by multiples of the last rendered viewport height.
    ///
    /// Negative values move toward the start. Clamps like [`scroll_lines`](Self::scroll_lines).
    pub fn scroll_pages(&mut self, pages: isize) {
        self.scroll_lines(pages.saturating_mul(self.height.max(1) as isize));
    }

    /// Move by half the last rendered viewport height, rounded up.
    ///
    /// Negative values move toward the start. Clamps like [`scroll_lines`](Self::scroll_lines).
    pub fn scroll_half_pages(&mut self, pages: isize) {
        self.scroll_lines(pages.saturating_mul(self.height.max(1).div_ceil(2) as isize));
    }

    /// Move horizontally by terminal cells; negative values move left.
    ///
    /// Both split panes share the offset. Movement is bounded by the last layout's content width
    /// and longest row, including headers. Ignored when the last render used wrapping.
    pub fn scroll_horizontal(&mut self, cells: isize) {
        if self.key.is_some_and(|k| k.wrap) {
            return;
        }
        self.horizontal = self.horizontal.saturating_add_signed(cells);
        self.horizontal = self
            .horizontal
            .min(self.max_width.saturating_sub(self.content_width));
    }

    /// Move to the first displayed row without changing the horizontal offset.
    pub fn start(&mut self) {
        self.offset = 0;
    }

    /// Move to the final full viewport without changing the horizontal offset.
    pub fn end(&mut self) {
        self.offset = self.screen.len();
        self.clamp();
    }

    /// Jump to the next original hunk header, opening its containing file if collapsed.
    /// At a closed file header this opens that file's first hunk.
    ///
    /// Leaves the viewport unchanged if none exists. The target is clamped to the final viewport.
    pub fn next_hunk(&mut self) {
        self.jump(true, false);
    }

    /// Jump to the last original hunk header above the top row, opening its file if collapsed.
    ///
    /// Inside a hunk, this returns to its header. At its header, this selects the preceding hunk.
    /// Leaves the viewport unchanged if none exists.
    pub fn previous_hunk(&mut self) {
        self.jump(false, false);
    }

    /// Jump to the first file header strictly below the top displayed row.
    ///
    /// Leaves the viewport unchanged if none exists. The target is clamped to the final viewport.
    pub fn next_file(&mut self) {
        self.jump(true, true);
    }

    /// Jump to the last file header strictly above the top displayed row.
    ///
    /// Inside a file, this returns to its header. At its header, this selects the preceding file.
    /// Leaves the viewport unchanged if none exists.
    pub fn previous_file(&mut self) {
        self.jump(false, true);
    }

    /// Map a zero-based absolute displayed row to a source line in the last rendered layout.
    ///
    /// Add [`offset`](Self::offset) to a viewport-relative row before calling. Returns `None` for
    /// out-of-range rows, headers, or a side without a source line. Wrapped continuations map to
    /// the same logical line, including blank padding beside a taller split partner.
    pub fn source_at(&self, displayed_row: usize, side: Side) -> Option<SourcePosition> {
        let row = self.rows.get(self.screen.get(displayed_row)?.row)?;
        if row.header {
            return None;
        }
        let line = match side {
            Side::Old => row.left.old,
            Side::New => row.right.as_ref().unwrap_or(&row.left).new,
        }?;
        Some(SourcePosition {
            file: row.file,
            side,
            line,
        })
    }

    /// Scroll to a source line's first displayed row in the last rendered layout.
    ///
    /// Returns `true` when found, even if final-viewport clamping prevents placing it at the top.
    /// Returns `false` without moving when the file or line is absent, the patch omits the line,
    /// or no layout has been rendered. Leaves the horizontal offset unchanged.
    /// Retained lines inside a collapsed fold expand and scroll on the next render.
    pub fn scroll_to_source(&mut self, position: SourcePosition) -> bool {
        if self.expand_source(position) {
            self.pending = Some(RevealTarget::Source(position));
            return true;
        }
        let index = match position.side {
            Side::Old => &self.old_sources,
            Side::New => &self.new_sources,
        };
        if let Ok(n) =
            index.binary_search_by_key(&(position.file, position.line), |&(f, l, _)| (f, l))
        {
            if self.folds_dirty {
                self.pending = Some(RevealTarget::Source(position));
            }
            self.offset = index[n].2;
            self.clamp();
            true
        } else {
            false
        }
    }

    // Keep a full final viewport when possible; a zero-height area still has a valid row offset.
    fn clamp(&mut self) {
        self.offset = self
            .offset
            .min(self.screen.len().saturating_sub(self.height.max(1)));
    }

    // File navigation keeps closed headers; hunk navigation opens the original target's file.
    fn jump(&mut self, forward: bool, files: bool) {
        if files {
            let target = if forward {
                self.files.iter().copied().find(|&row| row > self.offset)
            } else {
                self.files
                    .iter()
                    .copied()
                    .rev()
                    .find(|&row| row < self.offset)
            };
            if let Some(target) = target {
                self.offset = target;
                self.clamp();
            }
            return;
        }
        let target = if forward {
            self.hunks.iter().copied().find(|&(row, file, _)| {
                row > self.offset || (row == self.offset && self.collapsed_files.contains(&file))
            })
        } else {
            self.hunks
                .iter()
                .copied()
                .rev()
                .find(|&(row, _, _)| row < self.offset)
        };
        if let Some((row, file, logical)) = target {
            if self.collapsed_files.contains(&file) {
                let fold = self.file_folds[file];
                self.set_file_expanded(&fold, true);
                self.pending = Some(RevealTarget::Hunk {
                    file,
                    hunk: self.rows[logical].hunk.expect("indexed hunk header"),
                });
            } else {
                self.offset = row;
                self.clamp();
            }
        }
    }
}

/// A borrowed presentation of a prepared [`DiffDocument`].
///
/// Build this per frame or reuse it by shared reference. Render with
/// `Frame::render_stateful_widget(&diff, area, &mut state)` or the core
/// [`StatefulWidget::render`] method. Retain [`DiffState`] to preserve navigation and caches.
///
/// # Configuration
///
/// Use [`mode`](Self::mode) for unified/split layout, [`theme`](Self::theme) for styles,
/// [`line_numbers`](Self::line_numbers) and [`word_highlights`](Self::word_highlights) for
/// emphasis, and [`wrap`](Self::wrap), [`whitespace`](Self::whitespace), and
/// [`tab_width`](Self::tab_width) for text presentation. The [crate documentation](crate)
/// includes rendering and navigation examples.
#[derive(Debug, Clone, Copy)]
pub struct Diff<'a> {
    #[cfg(feature = "syntax")]
    syntax: Option<&'a crate::SyntaxStyles>,
    selection_style: Style,
    document: &'a DiffDocument,
    mode: ViewMode,
    theme: DiffTheme,
    words: bool,
    numbers: bool,
    wrap: bool,
    whitespace: bool,
    tab: usize,
    context: Option<usize>,
    show_stats: bool,
}

impl<'a> Diff<'a> {
    /// Borrow a prepared document using the default unified presentation.
    ///
    /// Defaults to the dark theme, visible line numbers and word highlights, four-cell tab stops,
    /// and no wrapping or whitespace markers. Construction does not prepare layout or change state.
    /// See the [crate example](crate) for rendering.
    pub fn new(document: &'a DiffDocument) -> Self {
        Self {
            document,
            #[cfg(feature = "syntax")]
            syntax: None,
            selection_style: Style::default().add_modifier(Modifier::REVERSED),
            mode: ViewMode::Unified,
            theme: DiffTheme::default(),
            words: true,
            numbers: true,
            wrap: false,
            whitespace: false,
            tab: 4,
            context: None,
            show_stats: false,
        }
    }

    /// Attach prepared syntax colors without changing layout or interaction state.
    ///
    /// Available with feature `syntax`. Prepare with [`crate::SyntaxHighlighter`] outside drawing;
    /// attachment only checks document identity and returns
    /// [`crate::SyntaxError::DocumentMismatch`] for results prepared against another document.
    /// Clones accept the same results. Syntax adds foreground/bold/italic before word,
    /// whitespace, search, and selection overlays. Unified context uses the new side without
    /// old-side fallback. Omit this option to disable syntax, including when a host chooses
    /// monochrome presentation.
    #[cfg(feature = "syntax")]
    pub fn syntax_styles(
        mut self,
        styles: &'a crate::SyntaxStyles,
    ) -> Result<Self, crate::SyntaxError> {
        styles.validate_document(self.document)?;
        self.syntax = Some(styles);
        Ok(self)
    }

    /// Fold retained unchanged runs, keeping this many lines beside each change.
    ///
    /// Defaults to `None`, showing all supplied source. `Some(0)` hides all retained unchanged
    /// lines; `Some(3)` keeps three at each change edge. This only changes presentation:
    /// unlike [`DiffDocument::compare`]'s context argument, it never discards source text.
    /// Supply `DiffDocument::compare(old, new, usize::MAX)` to retain all available context.
    /// Omitted patch context cannot be expanded. Changing the radius invalidates fold handles.
    ///
    /// ```
    /// use ratatui_core::buffer::Buffer;
    /// use ratatui_core::layout::Rect;
    /// use ratatui_core::widgets::StatefulWidget;
    /// use ratatui_diff::{Diff, DiffDocument, DiffState};
    ///
    /// let old = "one\ntwo\nthree\nfour\n";
    /// let new = "one\ntwo\nchanged\nfour\n";
    /// let document = DiffDocument::compare(old, new, usize::MAX);
    /// let widget = Diff::new(&document).context_lines(Some(0));
    /// let area = Rect::new(0, 0, 60, 10);
    /// let mut state = DiffState::new();
    /// (&widget).render(area, &mut Buffer::empty(area), &mut state);
    /// let fold = state.context_folds()[0].clone();
    /// assert_eq!(fold.line_count(), 2);
    /// assert!(state.set_context_expanded(&fold, true));
    /// (&widget).render(area, &mut Buffer::empty(area), &mut state);
    /// assert!(state.context_expanded(&fold));
    /// ```
    pub fn context_lines(mut self, radius: Option<usize>) -> Self {
        self.context = radius;
        self
    }

    /// Pin document-wide change totals above the scrollable diff. Disabled by default.
    ///
    /// Reserves one viewport row, including when all files are collapsed. Counts cover supplied
    /// insertion/deletion lines, independently of folding, wrapping, and view mode; they never
    /// estimate binary payload size or omitted source. Binary and metadata-only files are labeled
    /// separately. The summary has no source coordinates or hit target.
    ///
    /// ```
    /// use ratatui_diff::{Diff, DiffDocument};
    /// let document = DiffDocument::from_text("old\n", "new\n");
    /// let widget = Diff::new(&document).show_stats(true);
    /// ```
    pub fn show_stats(mut self, visible: bool) -> Self {
        self.show_stats = visible;
        self
    }

    /// Set the selection overlay, applied after line, word, and whitespace styles.
    ///
    /// Defaults to reverse video, which also works with monochrome themes. Does not affect layout.
    /// Future search overlays should be composed before selection.
    pub fn selection_style(mut self, style: Style) -> Self {
        self.selection_style = style;
        self
    }

    /// Select unified or split presentation. Defaults to [`ViewMode::Unified`].
    ///
    /// The next render rebuilds layout and attempts to retain the source position at the viewport.
    pub fn mode(mut self, mode: ViewMode) -> Self {
        self.mode = mode;
        self
    }

    /// Override display styles without invalidating layout.
    pub fn theme(mut self, theme: DiffTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Enable or disable word highlights without invalidating layout. Enabled by default.
    ///
    /// Disabling highlights retains whole-line styles and does not discard prepared ranges.
    pub fn word_highlights(mut self, enabled: bool) -> Self {
        self.words = enabled;
        self
    }

    /// Show or hide line numbers. Visible by default.
    ///
    /// Change markers remain adjacent to source text. Wrapped continuations use a dim `↪` in the
    /// number column instead of repeating the number. The next render recalculates gutters and
    /// wrapped rows.
    pub fn line_numbers(mut self, visible: bool) -> Self {
        self.numbers = visible;
        self
    }

    /// Wrap long lines without splitting graphemes or control-character escapes. Disabled by
    /// default.
    ///
    /// Enabling wrapping resets horizontal scrolling on the next render. Split pairs occupy the
    /// taller side's wrapped height. A grapheme or escape wider than the pane occupies a blank row.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Display spaces as `·` and tab starts as `→`. Disabled by default.
    ///
    /// Generated markers are dimmed over the source's row and word-highlight backgrounds; literal
    /// dots and arrows retain their source style. Tab padding fills the remaining cells to the next
    /// tab stop. Controls remain visible as escapes. Headers do not use whitespace markers.
    pub fn whitespace(mut self, visible: bool) -> Self {
        self.whitespace = visible;
        self
    }

    /// Set tab stops in terminal cells, measured from the start of each unwrapped source line.
    ///
    /// Defaults to four; zero is clamped to one. The gutter does not contribute to tab alignment.
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab = width.max(1);
        self
    }

    /// Retain a file-header or source anchor, rebuild layout, then restore and clamp the viewport.
    ///
    /// A topmost file header remains the anchor; otherwise use the first source line at or below
    /// the old offset, preferring the old side.
    /// Width, gutter, and wrapping changes reuse logical rows; mode, document, tab, or whitespace
    /// changes rebuild them. Height and styles do not invalidate geometry.
    fn prepare(self, area: Rect, state: &mut DiffState) {
        let key = LayoutKey {
            document: self.document.id,
            width: area.width,
            mode: self.mode,
            numbers: self.numbers,
            wrap: self.wrap,
            whitespace: self.whitespace,
            tab: self.tab,
            context: self.context,
        };
        if state
            .selection_document
            .is_some_and(|id| id != self.document.id)
        {
            state.clear_selection();
        }
        if state
            .search
            .document
            .is_some_and(|id| id != self.document.id)
        {
            state.clear_search();
        }
        if state.height != usize::from(area.height) {
            state.search.reveal |= state.search.active.is_some();
        }
        if state
            .key
            .is_some_and(|key| key.document != self.document.id)
        {
            state.pending = None;
        }
        if state.file_document != Some(self.document.id) {
            state.file_document = Some(self.document.id);
            state.collapsed_files.clear();
        }
        state.height = usize::from(area.height);
        if state.key == Some(key) && !state.folds_dirty {
            state.clamp();
            return;
        }

        if state.key != Some(key) {
            state.search.reveal |= state.search.active.is_some();
        }
        state.folds_dirty = false;

        // Screen offsets change with wrapping or mode; a source location survives both.
        let same_document = state.key.is_some_and(|k| k.document == key.document);
        let anchor = if same_document {
            let top = state
                .screen
                .get(state.offset)
                .map(|screen| &state.rows[screen.row]);
            if let Some(row) = top.filter(|row| row.file_header) {
                Some(RevealTarget::File(row.file))
            } else {
                (state.offset..state.screen.len())
                    .find_map(|n| {
                        state
                            .source_at(n, Side::Old)
                            .or_else(|| state.source_at(n, Side::New))
                    })
                    .map(RevealTarget::Source)
            }
        } else {
            None
        };
        let old_offset = state.offset;
        state.screen.clear();
        state.files.clear();
        state.hunks.clear();
        state.old_sources.clear();
        state.new_sources.clear();

        // Width changes reuse glyphs and alignment but still rebuild wrapped-row indexes.
        let rebuild_rows = state.key.is_none_or(|previous| {
            previous.document != key.document
                || previous.mode != key.mode
                || previous.whitespace != key.whitespace
                || previous.tab != key.tab
                || previous.context != key.context
        });
        if rebuild_rows {
            if state.key.is_none_or(|previous| {
                previous.document != key.document || previous.context != key.context
            }) {
                state.expanded.clear();
            }
            self.build_logical_rows(state);
        }
        self.index_screen_rows(area, state);

        // Replacing the document resets navigation; resizing retains the old source anchor.
        state.key = Some(key);
        state.offset = if same_document { old_offset } else { 0 };
        let target = state.pending.take().or(anchor);
        match target {
            Some(RevealTarget::Source(position)) => {
                let index = match position.side {
                    Side::Old => &state.old_sources,
                    Side::New => &state.new_sources,
                };
                if let Ok(n) = index
                    .binary_search_by_key(&(position.file, position.line), |&(file, line, _)| {
                        (file, line)
                    })
                {
                    state.offset = index[n].2;
                }
            }
            Some(RevealTarget::File(file)) => {
                state.offset = state.files[file];
            }
            Some(RevealTarget::Hunk { file, hunk }) => {
                let index = state.hunks.partition_point(|&(_, target_file, row)| {
                    (
                        target_file,
                        state.rows[row].hunk.expect("indexed hunk header"),
                    ) < (file, hunk)
                });
                if let Some(&(screen, _, _)) = state.hunks.get(index) {
                    state.offset = screen;
                }
            }
            None => {}
        }

        if self.wrap || !same_document {
            state.horizontal = 0;
        }
        state.horizontal = state
            .horizontal
            .min(state.max_width.saturating_sub(state.content_width));
        state.clamp();
    }

    /// Prepare width-independent source alignment and display glyphs.
    ///
    /// Each replacement run uses the same bounded similarity pairing as highlight refinement.
    /// Its shorter side receives empty content. Gutter digits use the largest source number in
    /// the whole document so scrolling does not change column alignment.
    fn build_logical_rows(self, state: &mut DiffState) {
        state.rows.clear();
        state.folds.clear();
        state.file_folds.clear();
        state.file_summaries.clear();
        let max_number = self
            .document
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .flat_map(|h| &h.lines)
            .flat_map(|l| [l.old, l.new])
            .flatten()
            .max()
            .unwrap_or(1);
        state.digits = max_number.to_string().len();
        for (file, source) in self.document.files.iter().enumerate() {
            state.file_folds.push(FileFold {
                file,
                document: self.document.id,
            });
            let summary = FileSummary::new(source);
            let label = summary.label(!state.collapsed_files.contains(&file), usize::MAX);
            state.file_summaries.push(summary);
            let mut file_header = header(file, None, &label, self.tab);
            file_header.file_header = true;
            state.rows.push(file_header);
            for text in &source.metadata {
                state.rows.push(header(file, None, text, self.tab));
            }
            if source.binary {
                state
                    .rows
                    .push(header(file, None, "Binary change", self.tab));
            }
            let mut old_end = 1;
            let mut new_end = 1;
            for (hunk, source) in source.hunks.iter().enumerate() {
                let old_start = source
                    .old
                    .start
                    .saturating_add(usize::from(source.old.is_empty()));
                let new_start = source
                    .new
                    .start
                    .saturating_add(usize::from(source.new.is_empty()));
                if old_start > old_end || new_start > new_end {
                    let mut gap = header(file, None, "… context unavailable", self.tab);
                    gap.gap = true;
                    state.rows.push(gap);
                }
                old_end = source
                    .old
                    .end
                    .saturating_add(usize::from(source.old.is_empty()));
                new_end = source
                    .new
                    .end
                    .saturating_add(usize::from(source.new.is_empty()));
                let label = format!(
                    "@@ -{},{} +{},{} @@",
                    source.old.start,
                    source.old.len(),
                    source.new.start,
                    source.new.len()
                );
                state.rows.push(header(file, Some(hunk), &label, self.tab));
                let mut n = 0;
                let mut hidden = None;
                while n < source.lines.len() {
                    let line = &source.lines[n];
                    if line.kind == LineKind::Context
                        && (n == 0 || source.lines[n - 1].kind != LineKind::Context)
                    {
                        hidden = self.context_fold(file, hunk, source, n, state);
                    }
                    let hidden_index = hidden
                        .filter(|&(_, start, stop)| n >= start && n < stop)
                        .map(|(index, _, _)| index);
                    if let Some((index, start, _)) = hidden
                        && n == start
                    {
                        let label = format!(
                            "… {} unchanged lines · expand",
                            state.folds[index].line_count()
                        );
                        let mut summary = header(file, Some(hunk), &label, self.tab);
                        summary.fold = Some(index);
                        state.rows.push(summary);
                    }
                    if self.mode == ViewMode::Unified || line.kind == LineKind::Context {
                        let left = content(line, self.whitespace, self.tab);
                        let right = (self.mode == ViewMode::Split)
                            .then(|| content(line, self.whitespace, self.tab));
                        state.rows.push(Row {
                            file,
                            hunk: Some(hunk),
                            left,
                            right,
                            header: false,
                            fold: None,
                            hidden: hidden_index,
                            gap: false,
                            file_header: false,
                        });
                        n += 1;
                    } else {
                        let end = source.lines[n..]
                            .iter()
                            .position(|l| l.kind == LineKind::Context)
                            .map_or(source.lines.len(), |x| n + x);
                        for (old, new) in crate::compare::replacement_pairs(&source.lines[n..end]) {
                            let left = old.map_or_else(empty, |i| {
                                content(&source.lines[n + i], self.whitespace, self.tab)
                            });
                            let right = new.map_or_else(empty, |i| {
                                content(&source.lines[n + i], self.whitespace, self.tab)
                            });
                            state.rows.push(Row {
                                file,
                                hunk: Some(hunk),
                                left,
                                right: Some(right),
                                header: false,
                                fold: None,
                                hidden: None,
                                gap: false,
                                file_header: false,
                            });
                        }
                        n = end;
                    }
                }
            }
        }
        let added: usize = state.file_summaries.iter().map(|file| file.added).sum();
        let removed: usize = state.file_summaries.iter().map(|file| file.removed).sum();
        let files = self.document.files.len();
        let noun = if files == 1 { "file" } else { "files" };
        state.statistics = format!("{files} {noun} · +{added} −{removed}");
        let binary = self
            .document
            .files
            .iter()
            .filter(|file| file.binary)
            .count();
        let metadata = self
            .document
            .files
            .iter()
            .filter(|file| !file.binary && file.hunks.is_empty() && !file.metadata.is_empty())
            .count();
        if binary != 0 {
            state.statistics.push_str(&format!(" · {binary} binary"));
        }
        if metadata != 0 {
            state
                .statistics
                .push_str(&format!(" · {metadata} metadata"));
        }
    }

    // Context runs are bounded by changes or hunk edges. Keep radius lines at each change,
    // then derive the hidden range from original numbering rather than displayed row counts.
    fn context_fold(
        self,
        file: usize,
        hunk: usize,
        source: &crate::Hunk,
        n: usize,
        state: &mut DiffState,
    ) -> Option<(usize, usize, usize)> {
        let radius = self.context?;
        let end = source.lines[n..]
            .iter()
            .position(|line| line.kind != LineKind::Context)
            .map_or(source.lines.len(), |length| n + length);
        let start = n.saturating_add(if n == 0 { 0 } else { radius }).min(end);
        let stop = end
            .saturating_sub(if end == source.lines.len() { 0 } else { radius })
            .max(n);
        if start >= stop {
            return None;
        }
        let first = &source.lines[start];
        let last = &source.lines[stop - 1];
        state.folds.push(ContextFold {
            file,
            hunk,
            old: first.old.unwrap()..last.old.unwrap() + 1,
            new: first.new.unwrap()..last.new.unwrap() + 1,
            document: self.document.id,
            radius,
        });
        Some((state.folds.len() - 1, start, stop))
    }

    /// Expand logical rows into screen rows and rebuild navigation indexes for this width.
    ///
    /// Requires prepared logical rows and cleared screen/navigation vectors. Source indexes point
    /// to the first wrapped row. Split pairs consume the taller side's height; empty glyph ranges
    /// pad shorter continuations without creating extra source lines.
    fn index_screen_rows(self, area: Rect, state: &mut DiffState) {
        state.max_width = 0;

        let mut previous_file = None;
        let mut previous_hunk = None;
        let expanded: Vec<_> = state
            .folds
            .iter()
            .map(|fold| state.expanded.contains(fold))
            .collect();
        // Synthetic controls are a single row. Short labels preserve count and action first.
        for row in &mut state.rows {
            if row.file_header {
                let label = state.file_summaries[row.file].label(
                    !state.collapsed_files.contains(&row.file),
                    usize::from(area.width),
                );
                row.left = header(row.file, None, &label, self.tab).left;
            }
            if let Some(index) = row.fold {
                let count = state.folds[index].line_count();
                let label = format!("… {count} unchanged lines · expand");
                let label = if label.width() > usize::from(area.width) {
                    format!("… {count} lines · expand")
                } else {
                    label
                };
                let label = if label.width() > usize::from(area.width) {
                    format!("{count} · expand")
                } else {
                    label
                };
                row.left = header(row.file, row.hunk, &label, self.tab).left;
            }
        }
        let mut folded_screen = 0;
        let mut file_screen = 0;
        for (n, row) in state.rows.iter().enumerate() {
            if row.file_header {
                file_screen = state.screen.len();
            } else if state.collapsed_files.contains(&row.file) {
                if let Some(hunk) = row.hunk
                    && previous_hunk != Some(hunk)
                {
                    state.hunks.push((file_screen, row.file, n));
                    previous_hunk = Some(hunk);
                }
                if let Some(line) = row.left.old {
                    state.old_sources.push((row.file, line, file_screen));
                }
                if let Some(line) = row.right.as_ref().unwrap_or(&row.left).new {
                    state.new_sources.push((row.file, line, file_screen));
                }
                continue;
            }
            if let Some(index) = row.fold {
                if expanded[index] {
                    continue;
                }
                folded_screen = state.screen.len();
            }
            if let Some(index) = row.hidden
                && !expanded[index]
            {
                if let Some(line) = row.left.old {
                    state.old_sources.push((row.file, line, folded_screen));
                }
                if let Some(line) = row.right.as_ref().unwrap_or(&row.left).new {
                    state.new_sources.push((row.file, line, folded_screen));
                }
                continue;
            }
            let width = self
                .pane_width(area.width, row.header)
                .saturating_sub(self.gutter_width(state.digits, row.header))
                .max(1);
            state.content_width = width;
            state.max_width = state
                .max_width
                .max(row.left.width)
                .max(row.right.as_ref().map_or(0, |c| c.width));
            let left = segments(
                &row.left,
                width,
                self.wrap && row.fold.is_none() && !row.gap && !row.file_header,
            );
            let right = row
                .right
                .as_ref()
                .map_or_else(Vec::new, |c| segments(c, width, self.wrap));

            // Record boundaries before expansion so jumps land on headers or first continuations.
            let start = state.screen.len();
            if previous_file != Some(row.file) {
                state.files.push(start);
                previous_file = Some(row.file);
                previous_hunk = None;
            }
            if let Some(h) = row.hunk
                && previous_hunk != Some(h)
            {
                state.hunks.push((start, row.file, n));
                previous_hunk = Some(h);
            }
            if let Some(l) = row.left.old {
                state.old_sources.push((row.file, l, start));
            }
            if let Some(l) = row.right.as_ref().unwrap_or(&row.left).new {
                state.new_sources.push((row.file, l, start));
            }
            for i in 0..left.len().max(right.len()) {
                state.screen.push(ScreenRow {
                    row: n,
                    left: left.get(i).cloned().unwrap_or(0..0),
                    right: right.get(i).cloned().unwrap_or(0..0),
                    continuation: i > 0,
                });
            }
        }

        // Source lookup is independent of displayed ordering and uses binary search.
        state.old_sources.sort_unstable();
        state.new_sources.sort_unstable();
    }

    // Equal source widths reserve one separator cell and, at even widths, right-edge padding.
    fn pane_width(self, width: u16, header: bool) -> usize {
        if self.mode == ViewMode::Split && !header {
            usize::from(width.saturating_sub(1)) / 2
        } else {
            usize::from(width)
        }
    }

    // Width must match the prefixes in `draw`: source numbers, change marker, and spaces.
    fn gutter_width(self, digits: usize, header: bool) -> usize {
        if header {
            0
        } else if !self.numbers {
            1
        } else if self.mode == ViewMode::Unified {
            digits * 2 + 3
        } else {
            digits + 2
        }
    }
}

impl StatefulWidget for &Diff<'_> {
    type State = DiffState;

    /// Prepare layout, update navigation state, and paint the visible rows.
    ///
    /// Clears symbols and styles throughout `area`, including cells beyond the document. Reuses
    /// cached geometry when possible; changing document identity resets navigation. As with other
    /// Ratatui widgets, `area` must lie within the supplied buffer.
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut DiffState) {
        let full_area = area;
        let area = if self.show_stats && area.height != 0 {
            Rect::new(
                area.x,
                area.y.saturating_add(1),
                area.width,
                area.height - 1,
            )
        } else {
            area
        };
        self.prepare(area, state);
        let reveal = if state.reveal_selection {
            state.selection.map(|selection| selection.focus.position)
        } else if state.search.reveal {
            state
                .search
                .active
                .map(|index| state.search.matches[index].position)
        } else {
            None
        };
        if let Some(position) = reveal
            && state.expand_source(position)
        {
            self.prepare(area, state);
            if !state.reveal_selection {
                state.search.reveal = true;
            }
        }
        if std::mem::take(&mut state.reveal_selection) {
            state.search.reveal = false;
            if let Some(selection) = state.selection {
                state.reveal_boundary(selection.focus.position, selection.focus.byte, true);
            }
        } else {
            state.reveal_match();
        }
        state.rendered = Some((area, state.offset, state.horizontal));
        buf.set_style(full_area, self.theme.context);
        for y in full_area.y..full_area.bottom() {
            for x in full_area.x..full_area.right() {
                buf[(x, y)].reset();
                buf[(x, y)].set_style(self.theme.context);
            }
        }
        if self.show_stats && full_area.height != 0 {
            buf.set_stringn(
                full_area.x,
                full_area.y,
                &state.statistics,
                usize::from(full_area.width),
                self.theme.header,
            );
            self.color_counts(
                buf,
                full_area.x,
                full_area.y,
                usize::from(full_area.width),
                &state.statistics,
                0,
            );
        }
        for (y, screen) in state
            .screen
            .iter()
            .skip(state.offset)
            .take(usize::from(area.height))
            .enumerate()
        {
            let row = &state.rows[screen.row];
            let pane_width = self.pane_width(area.width, row.header);
            let gutter = self.gutter_width(state.digits, row.header).min(pane_width);
            let width = pane_width.saturating_sub(gutter);
            let py = area.y + y as u16;
            let painter = if row.fold.is_some() || row.gap {
                Diff {
                    theme: DiffTheme {
                        header: self.theme.gutter,
                        ..self.theme
                    },
                    ..*self
                }
            } else {
                *self
            };
            painter.draw(
                &row.left,
                screen,
                area.x,
                py,
                pane_width,
                gutter,
                width,
                row.header,
                Side::Old,
                buf,
                state,
            );
            if row.file_header {
                let counts = &state.file_summaries[row.file].changes;
                let start = row.left.width.saturating_sub(counts.width());
                self.color_counts(buf, area.x, py, usize::from(area.width), counts, start);
            }
            if let Some(right) = &row.right {
                let x = area.x + pane_width as u16;
                if x < area.right() {
                    buf[(x, py)].set_symbol("│").set_style(self.theme.gutter);
                }
                let right_x = x.saturating_add(1);
                let right_pane = usize::from(area.right().saturating_sub(right_x));
                self.draw(
                    right,
                    screen,
                    right_x,
                    py,
                    right_pane,
                    gutter,
                    width,
                    false,
                    Side::New,
                    buf,
                    state,
                );
            }
        }
    }
}

impl Diff<'_> {
    /// Color only count tokens, retaining the header background and modifiers.
    ///
    /// File paths are excluded by the caller, so `+` or `−` in a path cannot acquire diff colors.
    /// Foregrounds come from line roles; tinted source-row backgrounds do not leak into headers.
    fn color_counts(
        self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: usize,
        text: &str,
        start: usize,
    ) {
        let mut column = start;
        for token in text.split_inclusive(' ') {
            let foreground = if token.starts_with('+') {
                self.theme.insert.fg
            } else if token.starts_with('−') {
                self.theme.delete.fg
            } else {
                None
            };
            if let Some(fg) = foreground {
                for cell in column..(column + token.trim_end().width()).min(width) {
                    buf[(x + cell as u16, y)].set_fg(fg);
                }
            }
            column += token.width();
        }
    }

    // Paint one pane's glyph range. x/y are buffer coordinates; pane/gutter/width are cell counts.
    // The segment indexes prepared glyphs. Geometry comes from the same layout used for wrapping.
    #[allow(clippy::too_many_arguments)] // Geometry is explicit at the sole cell-writing boundary.
    fn draw(
        self,
        content: &Content,
        screen: &ScreenRow,
        x: u16,
        y: u16,
        pane: usize,
        gutter: usize,
        width: usize,
        header: bool,
        side: Side,
        buf: &mut Buffer,
        state: &DiffState,
    ) {
        let segment = match side {
            Side::Old => &screen.left,
            Side::New => &screen.right,
        };
        let style = if header {
            self.theme.header
        } else {
            match content.kind {
                LineKind::Context => self.theme.context,
                LineKind::Insert => self.theme.insert,
                LineKind::Delete => self.theme.delete,
            }
        };
        if pane == 0 || x >= buf.area.right() {
            return;
        }
        for dx in 0..pane.min(usize::from(buf.area.right() - x)) {
            buf[(x + dx as u16, y)].set_style(style);
        }
        if !header && !(screen.continuation && segment.is_empty()) {
            let marker = match content.kind {
                LineKind::Context => ' ',
                LineKind::Insert => '+',
                LineKind::Delete => '-',
            };
            let number = |n: Option<usize>, side: Side| {
                let Some(n) = n else {
                    return " ".repeat(state.digits);
                };
                if screen.continuation {
                    // Unified context has two numbers but needs only one continuation cue.
                    if self.mode == ViewMode::Unified && side == Side::Old && content.new.is_some()
                    {
                        " ".repeat(state.digits)
                    } else {
                        format!("{:>w$}", "↪", w = state.digits)
                    }
                } else {
                    format!("{n:>w$}", w = state.digits)
                }
            };
            let prefix = if !self.numbers {
                format!("{marker}")
            } else if self.mode == ViewMode::Unified {
                format!(
                    "{} {} {marker}",
                    number(content.old, Side::Old),
                    number(content.new, Side::New)
                )
            } else {
                format!(
                    "{} {marker}",
                    number(
                        if side == Side::Old {
                            content.old
                        } else {
                            content.new
                        },
                        side
                    )
                )
            };
            buf.set_stringn(x, y, prefix, gutter, self.theme.gutter);
            if screen.continuation {
                for column in 0..gutter {
                    let cell = &mut buf[(x + column as u16, y)];
                    if cell.symbol() == "↪" {
                        let cue = self.theme.context.patch(style).patch(self.theme.gutter);
                        cell.set_style(ghost_style(cue, 2));
                    }
                }
            }
            let marker_column = self.gutter_width(state.digits, false) - 1;
            if marker_column < gutter {
                let cell = &mut buf[(x + marker_column as u16, y)];
                cell.reset();
                cell.set_char(marker)
                    .set_style(self.theme.context.patch(style));
            }
        }
        if width == 0 || segment.is_empty() {
            return;
        }

        // Wrapped segments start at their own column; unwrapped segments share the viewport offset.
        let synthetic = &state.rows[screen.row];
        let base = if synthetic.fold.is_some() || synthetic.gap || synthetic.file_header {
            0
        } else if self.wrap {
            content.glyphs[segment.start].column
        } else {
            state.horizontal
        };
        let selected_line = state.selection.and_then(|selection| {
            let selected_side = selection.anchor.position.side;
            if self.mode == ViewMode::Split && selected_side != side {
                return None;
            }
            let number = match selected_side {
                Side::Old => content.old,
                Side::New => content.new,
            }?;
            Some((
                selection,
                SourcePosition {
                    file: state.rows[screen.row].file,
                    side: selected_side,
                    line: number,
                },
            ))
        });
        // Skip off-screen glyphs by column, then draw only complete graphemes inside the pane.
        let first = content.glyphs[segment.clone()].partition_point(|g| g.column + g.width <= base)
            + segment.start;
        let row = &state.rows[screen.row];
        let source_side = if self.mode == ViewMode::Unified && content.new.is_some() {
            Side::New
        } else {
            side
        };
        let source_line = match source_side {
            Side::Old => content.old,
            Side::New => content.new,
        };
        let candidates = source_line.map_or(&[][..], |line| {
            state.search.indexes(SourcePosition {
                file: row.file,
                side: source_side,
                line,
            })
        });
        #[cfg(feature = "syntax")]
        let syntax = source_line.map_or(&[][..], |line| {
            self.syntax.map_or(&[][..], |styles| {
                styles.line(SourcePosition {
                    file: row.file,
                    side: source_side,
                    line,
                })
            })
        });
        #[cfg(feature = "syntax")]
        let mut syntax_index = content
            .glyphs
            .get(first)
            .and_then(|glyph| glyph.bytes.as_ref())
            .map_or(0, |bytes| {
                syntax.partition_point(|span| span.bytes.end <= bytes.start)
            });
        // Unified context can be found through either selected side.
        let candidates = if candidates.is_empty()
            && self.mode == ViewMode::Unified
            && let Some(line) = content.old
        {
            state.search.indexes(SourcePosition {
                file: row.file,
                side: Side::Old,
                line,
            })
        } else {
            candidates
        };
        for glyph in &content.glyphs[first..segment.end] {
            if glyph.column < base {
                continue;
            } // A clipped wide grapheme is left blank.
            let column = glyph.column - base;
            if column + glyph.width > width {
                break;
            }
            #[cfg(feature = "syntax")]
            let style = glyph.bytes.as_ref().map_or(style, |bytes| {
                while syntax
                    .get(syntax_index)
                    .is_some_and(|span| span.bytes.end <= bytes.start)
                {
                    syntax_index += 1;
                }
                syntax
                    .get(syntax_index)
                    .filter(|span| span.bytes.start < bytes.end)
                    .map_or(style, |span| style.patch(span.style))
            });
            let emphasized = if content.kind == LineKind::Insert {
                self.theme.insert_word
            } else {
                self.theme.delete_word
            };
            let style = if self.words && glyph.emphasized {
                style.patch(emphasized)
            } else {
                style
            };
            let style = if glyph.whitespace_marker {
                ghost_style(self.theme.context.patch(style), 4)
            } else {
                style
            };
            let mut search_style = None;
            if let Some(bytes) = &glyph.bytes {
                let end = candidates.partition_point(|&(_, _, _, n)| {
                    state.search.matches[n].bytes.end <= bytes.start
                });
                let hits = candidates[end..]
                    .iter()
                    .take_while(|&&(_, _, _, n)| state.search.matches[n].bytes.start < bytes.end);
                for &(_, _, _, n) in hits {
                    search_style = Some(if state.search.active == Some(n) {
                        self.theme.search_active
                    } else {
                        self.theme.search_match
                    });
                    if state.search.active == Some(n) {
                        break;
                    }
                }
            }
            let style = search_style.map_or(style, |highlight| style.patch(highlight));
            let style = if glyph.bytes.as_ref().is_some_and(|bytes| {
                selected_line
                    .is_some_and(|(selection, position)| selection.intersects(position, bytes))
            }) {
                style.patch(self.selection_style)
            } else {
                style
            };
            buf.set_stringn(
                x + (gutter + column) as u16,
                y,
                &content.text[glyph.text.clone()],
                glyph.width,
                style,
            );
            // Ratatui resets wide-grapheme continuation cells while writing the symbol.
            for cell in 0..glyph.width {
                buf[(x + (gutter + column + cell) as u16, y)].set_style(style);
            }
        }
    }
}

// Synthetic cues recede toward their composed background. RGB colors make the fraction explicit;
// palette-owned colors retain the terminal's dim treatment because their actual RGB is unknown.
fn ghost_style(style: Style, divisor: u16) -> Style {
    let (Some(Color::Rgb(r, g, b)), Some(Color::Rgb(br, bg, bb))) = (style.fg, style.bg) else {
        return style.add_modifier(Modifier::DIM);
    };
    let blend = |foreground: u8, background: u8| {
        ((u16::from(foreground) + u16::from(background) * (divisor - 1)) / divisor) as u8
    };
    style.fg(Color::Rgb(blend(r, br), blend(g, bg), blend(b, bb)))
}

// Padding content has no source identity, marker, or missing-newline annotation.
fn empty() -> Content {
    Content {
        text: String::new(),
        glyphs: Box::default(),
        kind: LineKind::Context,
        old: None,
        new: None,
        width: 0,
    }
}

// Headers span both panes and reuse control escaping without source numbers or whitespace marks.
fn header(file: usize, hunk: Option<usize>, text: &str, tab: usize) -> Row {
    let line = DiffLine::new(LineKind::Context, None, None, text);
    Row {
        file,
        hunk,
        left: content(&line, false, tab),
        right: None,
        header: true,
        fold: None,
        hidden: None,
        gap: false,
        file_header: false,
    }
}

/// Convert source graphemes to display glyphs with cumulative terminal-cell columns.
///
/// `tab` must be nonzero. Tabs expand to the next stop, controls become visible escapes, and
/// standalone zero-width clusters receive a dotted-circle base. Source highlight byte ranges are
/// mapped before expansion; wrapping and clipping then share the prepared geometry.
fn content(line: &DiffLine, whitespace: bool, tab: usize) -> Content {
    let mut text = line.text.clone();
    let mut glyphs = Vec::new();
    let mut column = 0;
    for (byte, g) in line.text.grapheme_indices(true) {
        let emphasized = line.highlights.as_ref().is_some_and(|ranges| {
            ranges
                .iter()
                .any(|r| r.start < byte + g.len() && r.end > byte)
        });
        if g == "\t" {
            let width = tab - column % tab;
            for n in 0..width {
                glyphs.push(Glyph {
                    text: append_glyph_text(
                        &mut text,
                        if whitespace && n == 0 { "→" } else { " " },
                    ),
                    bytes: Some(byte..byte + g.len()),
                    column,
                    width: 1,
                    emphasized,
                    whitespace_marker: whitespace && n == 0,
                });
                column += 1;
            }
        } else {
            let mut display = if g.chars().any(char::is_control) {
                let escaped = g
                    .chars()
                    .map(|c| {
                        if c.is_control() {
                            format!("\\u{{{:x}}}", c as u32)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>();
                append_glyph_text(&mut text, &escaped)
            } else if whitespace && g == " " {
                append_glyph_text(&mut text, "·")
            } else {
                byte..byte + g.len()
            };
            let width = text[display.clone()].width();
            // A standalone zero-width cluster must not attach to the gutter or previous cell.
            let width = if width == 0 {
                display = append_glyph_text(&mut text, &format!("◌{g}"));
                1
            } else {
                width
            };
            glyphs.push(Glyph {
                text: display,
                bytes: Some(byte..byte + g.len()),
                column,
                width,
                emphasized,
                whitespace_marker: whitespace && g == " ",
            });
            column += width;
        }
    }

    // End-of-file notation is presentation only and has no source highlight range.
    if !line.terminated && (line.old.is_some() || line.new.is_some()) {
        for g in " ⏎".graphemes(true) {
            let width = g.width();
            glyphs.push(Glyph {
                text: append_glyph_text(&mut text, g),
                bytes: None,
                column,
                width,
                emphasized: false,
                whitespace_marker: false,
            });
            column += width;
        }
    }
    Content {
        text,
        // Completed glyph geometry never grows. Discard preparation's geometric Vec slack.
        glyphs: glyphs.into_boxed_slice(),
        kind: line.kind,
        old: line.old,
        new: line.new,
        width: column,
    }
}

// Appended display text cannot change the byte offsets of the immutable source prefix.
fn append_glyph_text(text: &mut String, glyph: &str) -> Range<usize> {
    let start = text.len();
    text.push_str(glyph);
    start..text.len()
}

/// Greedily pack whole glyphs into ranges of glyph indexes.
///
/// Empty content still produces one range, preserving blank source lines. A glyph wider than the
/// pane occupies a row by itself; drawing leaves it blank without splitting it or stalling.
/// With wrapping disabled, the single returned range covers all glyphs.
fn segments(content: &Content, width: usize, wrap: bool) -> Vec<Range<usize>> {
    if !wrap || content.glyphs.is_empty() {
        return std::iter::once(0..content.glyphs.len()).collect();
    }
    let mut out = Vec::new();
    let mut start = 0;
    let mut used = 0;
    for (i, glyph) in content.glyphs.iter().enumerate() {
        if used > 0 && used + glyph.width > width {
            out.push(start..i);
            start = i;
            used = 0;
        }
        used += glyph.width;
    }
    out.push(start..content.glyphs.len());
    out
}

#[cfg(test)]
mod glyph_memory_tests {
    use super::*;

    #[test]
    fn prepared_long_unicode_line_has_bounded_text_storage() {
        let source = "界e\u{301}👩‍💻 hello".repeat(10_000);
        let mut line = DiffLine::new(LineKind::Insert, None, Some(1), &source);
        line.terminated = true;
        let prepared = content(&line, false, 4);
        let displayed: String = prepared
            .glyphs
            .iter()
            .map(|glyph| &prepared.text[glyph.text.clone()])
            .collect();
        assert_eq!(displayed, source);
        assert!(prepared.text.capacity() <= source.len() * 2);
        assert_eq!(prepared.width, source.width());
    }

    #[test]
    fn transformed_glyphs_preserve_source_ranges_and_units() {
        let source = "\u{301}\t界e\u{301}👩‍💻 \u{1}";
        let mut line = DiffLine::new(LineKind::Insert, None, Some(1), source);
        line.terminated = false;
        let prepared = content(&line, true, 4);
        let displayed: String = prepared
            .glyphs
            .iter()
            .map(|glyph| &prepared.text[glyph.text.clone()])
            .collect();
        assert_eq!(displayed, "◌\u{301}→  界e\u{301}👩‍💻·\\u{1} ⏎");
        let tab: Vec<_> = prepared
            .glyphs
            .iter()
            .filter(|g| g.bytes == Some(2..3))
            .collect();
        assert_eq!(tab.len(), 3);
        assert!(tab.iter().all(|g| g.width == 1));
        let emoji = prepared
            .glyphs
            .iter()
            .find(|g| g.bytes == Some(9..20))
            .unwrap();
        assert_eq!(emoji.width, 2);
        assert_eq!(&prepared.text[emoji.text.clone()], "👩‍💻");
        let control = prepared
            .glyphs
            .iter()
            .find(|g| g.bytes == Some(21..22))
            .unwrap();
        assert_eq!(control.width, 5);
        assert_eq!(&prepared.text[control.text.clone()], "\\u{1}");
        assert_eq!(
            prepared.glyphs.iter().filter(|g| g.bytes.is_none()).count(),
            2
        );
    }
}
