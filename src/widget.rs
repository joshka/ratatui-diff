//! Prepared source rows, width-dependent screen rows, and viewport navigation.
//!
//! [`Diff`] borrows immutable content; [`DiffState`] owns its cached geometry and offsets.
//! Preparation may traverse the document. Drawing uses the indexed visible rows.

use std::ops::Range;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{DiffDocument, DiffLine, DiffTheme, LineKind, Side};

/// Diff presentation mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Original and modified lines in unified order, with both line-number columns. The default.
    #[default]
    Unified,

    /// Original source on the left and modified source on the right.
    ///
    /// Replacement lines pair in source order; unmatched lines leave a blank opposite pane.
    /// Wrapped pairs use the taller side's height, and horizontal scrolling moves both panes.
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
/// Public field construction is unchecked; hit-testing returns valid grapheme ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    /// File, side, and one-based line identity.
    pub position: SourcePosition,

    /// Complete source grapheme bytes; expanded tabs and escapes share this range.
    pub bytes: Range<usize>,
}

/// The semantic region occupying a cell in the last rendered diff viewport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTest {
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
    /// A file, metadata, or hunk header.
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
    text: String,
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
    glyphs: Vec<Glyph>,
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
    offset: usize,
    horizontal: usize,
    height: usize,
    key: Option<LayoutKey>,
    rows: Vec<Row>,
    screen: Vec<ScreenRow>,
    // Absolute screen-row indexes of file and hunk headers, in display order.
    files: Vec<usize>,
    hunks: Vec<usize>,
    // Sorted (file index, source line, first screen row) tuples for binary lookup.
    old_sources: Vec<(usize, usize, usize)>,
    new_sources: Vec<(usize, usize, usize)>,
    digits: usize,
    max_width: usize,
    content_width: usize,
}

impl DiffState {
    /// Inspect the current source selection. It survives layout changes, but not document
    /// replacement.
    pub fn selection(&self) -> Option<crate::SourceSelection> {
        self.selection
    }

    /// Set anchor/focus from host keyboard or pointer input.
    ///
    /// Rejects invalid source boundaries, cross-file/side ranges, and omitted patch context.
    /// The selection belongs to `document`; rendering another document clears it.
    pub fn set_selection(
        &mut self,
        document: &DiffDocument,
        selection: crate::SourceSelection,
    ) -> bool {
        if selection.text(document).is_none() {
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

    /// Jump to the first hunk header strictly below the top displayed row.
    ///
    /// Leaves the viewport unchanged if none exists. The target is clamped to the final viewport.
    pub fn next_hunk(&mut self) {
        self.jump(true, false);
    }

    /// Jump to the last hunk header strictly above the top displayed row.
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
    pub fn scroll_to_source(&mut self, position: SourcePosition) -> bool {
        let index = match position.side {
            Side::Old => &self.old_sources,
            Side::New => &self.new_sources,
        };
        if let Ok(n) =
            index.binary_search_by_key(&(position.file, position.line), |&(f, l, _)| (f, l))
        {
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

    // Strict comparisons skip the current header; clamping may leave a later target below the top.
    fn jump(&mut self, forward: bool, files: bool) {
        let targets = if files { &self.files } else { &self.hunks };
        let n = if forward {
            targets.iter().copied().find(|&n| n > self.offset)
        } else {
            targets.iter().copied().rev().find(|&n| n < self.offset)
        };
        if let Some(n) = n {
            self.offset = n;
            self.clamp();
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
    selection_style: Style,
    document: &'a DiffDocument,
    mode: ViewMode,
    theme: DiffTheme,
    words: bool,
    numbers: bool,
    wrap: bool,
    whitespace: bool,
    tab: usize,
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
            selection_style: Style::default().add_modifier(Modifier::REVERSED),
            mode: ViewMode::Unified,
            theme: DiffTheme::default(),
            words: true,
            numbers: true,
            wrap: false,
            whitespace: false,
            tab: 4,
        }
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

    /// Retain a source anchor, rebuild invalidated layout, then restore and clamp the viewport.
    ///
    /// The anchor is the first source line at or below the old offset, preferring the old side.
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
        };
        if state
            .selection_document
            .is_some_and(|id| id != self.document.id)
        {
            state.clear_selection();
        }
        state.height = usize::from(area.height);
        if state.key == Some(key) {
            state.clamp();
            return;
        }

        // Screen offsets change with wrapping or mode; a source location survives both.
        let same_document = state.key.is_some_and(|k| k.document == key.document);
        let anchor = if same_document {
            (state.offset..state.screen.len()).find_map(|n| {
                state
                    .source_at(n, Side::Old)
                    .or_else(|| state.source_at(n, Side::New))
            })
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
        });
        if rebuild_rows {
            self.build_logical_rows(state);
        }
        self.index_screen_rows(area, state);

        // Replacing the document resets navigation; resizing retains the old source anchor.
        state.key = Some(key);
        state.offset = if same_document { old_offset } else { 0 };
        if let Some(anchor) = anchor {
            state.scroll_to_source(anchor);
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
    /// Each replacement run pairs deletions and insertions in source order, matching refinement.
    /// Its shorter side receives empty content. Gutter digits use the largest source number in
    /// the whole document so scrolling does not change column alignment.
    fn build_logical_rows(self, state: &mut DiffState) {
        state.rows.clear();
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
            let label = format!(
                "{} → {}",
                source.old_path.as_deref().unwrap_or("/dev/null"),
                source.new_path.as_deref().unwrap_or("/dev/null")
            );
            state.rows.push(header(file, None, &label, self.tab));
            for text in &source.metadata {
                state.rows.push(header(file, None, text, self.tab));
            }
            if source.binary {
                state
                    .rows
                    .push(header(file, None, "Binary change", self.tab));
            }
            for (hunk, source) in source.hunks.iter().enumerate() {
                let label = format!(
                    "@@ -{},{} +{},{} @@",
                    source.old.start,
                    source.old.len(),
                    source.new.start,
                    source.new.len()
                );
                state.rows.push(header(file, Some(hunk), &label, self.tab));
                let mut n = 0;
                while n < source.lines.len() {
                    let line = &source.lines[n];
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
                        });
                        n += 1;
                    } else {
                        let end = source.lines[n..]
                            .iter()
                            .position(|l| l.kind == LineKind::Context)
                            .map_or(source.lines.len(), |x| n + x);
                        let deletes: Vec<_> = source.lines[n..end]
                            .iter()
                            .filter(|l| l.kind == LineKind::Delete)
                            .collect();
                        let inserts: Vec<_> = source.lines[n..end]
                            .iter()
                            .filter(|l| l.kind == LineKind::Insert)
                            .collect();
                        for i in 0..deletes.len().max(inserts.len()) {
                            let left = deletes
                                .get(i)
                                .map_or_else(empty, |l| content(l, self.whitespace, self.tab));
                            let right = inserts
                                .get(i)
                                .map_or_else(empty, |l| content(l, self.whitespace, self.tab));
                            state.rows.push(Row {
                                file,
                                hunk: Some(hunk),
                                left,
                                right: Some(right),
                                header: false,
                            });
                        }
                        n = end;
                    }
                }
            }
        }
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
        for (n, row) in state.rows.iter().enumerate() {
            let width = self
                .pane_width(area.width, row.header)
                .saturating_sub(self.gutter_width(state.digits, row.header))
                .max(1);
            state.content_width = width;
            state.max_width = state
                .max_width
                .max(row.left.width)
                .max(row.right.as_ref().map_or(0, |c| c.width));
            let left = segments(&row.left, width, self.wrap);
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
                state.hunks.push(start);
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
        self.prepare(area, state);
        state.rendered = Some((area, state.offset, state.horizontal));
        buf.set_style(area, self.theme.context);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buf[(x, y)].reset();
                buf[(x, y)].set_style(self.theme.context);
            }
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
            self.draw(
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
        let base = if self.wrap {
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
        for glyph in &content.glyphs[first..segment.end] {
            if glyph.column < base {
                continue;
            } // A clipped wide grapheme is left blank.
            let column = glyph.column - base;
            if column + glyph.width > width {
                break;
            }
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
                &glyph.text,
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
        glyphs: Vec::new(),
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
    }
}

/// Convert source graphemes to display glyphs with cumulative terminal-cell columns.
///
/// `tab` must be nonzero. Tabs expand to the next stop, controls become visible escapes, and
/// standalone zero-width clusters receive a dotted-circle base. Source highlight byte ranges are
/// mapped before expansion; wrapping and clipping then share the prepared geometry.
fn content(line: &DiffLine, whitespace: bool, tab: usize) -> Content {
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
                    text: if whitespace && n == 0 {
                        "→".into()
                    } else {
                        " ".into()
                    },
                    bytes: Some(byte..byte + g.len()),
                    column,
                    width: 1,
                    emphasized,
                    whitespace_marker: whitespace && n == 0,
                });
                column += 1;
            }
        } else {
            let text = if g.chars().any(char::is_control) {
                g.chars()
                    .map(|c| {
                        if c.is_control() {
                            format!("\\u{{{:x}}}", c as u32)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>()
            } else if whitespace && g == " " {
                "·".into()
            } else {
                g.to_owned()
            };
            let width = UnicodeWidthStr::width(text.as_str());
            // Standalone zero-width clusters get a dotted-circle base, avoiding attachment
            // to a gutter or unrelated previous cell.
            let (text, width) = if width == 0 {
                (format!("◌{text}"), 1)
            } else {
                (text, width)
            };
            glyphs.push(Glyph {
                text,
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
                text: g.into(),
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
        glyphs,
        kind: line.kind,
        old: line.old,
        new: line.new,
        width: column,
    }
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
