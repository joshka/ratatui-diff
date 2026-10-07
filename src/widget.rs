//! Indexed screen rows keep frame work independent of total document length.
use std::ops::Range;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{DiffDocument, DiffLine, DiffTheme, LineKind, Side};

/// Diff presentation mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Interleaved original and modified lines.
    #[default]
    Unified,
    /// Original and modified source in synchronized columns.
    Split,
}
/// A numbered source location within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// File index, starting at zero.
    pub file: usize,
    /// Original or modified source.
    pub side: Side,
    /// Source line number, starting at one.
    pub line: usize,
}
#[derive(Debug)]
struct Glyph {
    text: String,
    column: usize,
    width: usize,
    emphasized: bool,
}
#[derive(Debug)]
struct Content {
    glyphs: Vec<Glyph>,
    kind: LineKind,
    old: Option<usize>,
    new: Option<usize>,
    width: usize,
}
#[derive(Debug)]
struct Row {
    file: usize,
    hunk: Option<usize>,
    left: Content,
    right: Option<Content>,
    header: bool,
}
#[derive(Debug)]
struct ScreenRow {
    row: usize,
    left: Range<usize>,
    right: Range<usize>,
}
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
#[derive(Debug, Default)]
pub struct DiffState {
    offset: usize,
    horizontal: usize,
    height: usize,
    key: Option<LayoutKey>,
    rows: Vec<Row>,
    screen: Vec<ScreenRow>,
    files: Vec<usize>,
    hunks: Vec<usize>,
    old_sources: Vec<(usize, usize, usize)>,
    new_sources: Vec<(usize, usize, usize)>,
    digits: usize,
    max_width: usize,
    content_width: usize,
}
impl DiffState {
    /// Construct an empty viewport.
    pub fn new() -> Self {
        Self::default()
    }
    /// Top displayed row.
    pub fn offset(&self) -> usize {
        self.offset
    }
    /// Horizontal cell offset, shared between split panes.
    pub fn horizontal_offset(&self) -> usize {
        self.horizontal
    }
    /// Total displayed rows for the last rendered layout.
    pub fn row_count(&self) -> usize {
        self.screen.len()
    }
    /// Move by displayed rows; negative values move toward the start.
    pub fn scroll_lines(&mut self, lines: isize) {
        self.offset = self.offset.saturating_add_signed(lines);
        self.clamp();
    }
    /// Move by viewport pages.
    pub fn scroll_pages(&mut self, pages: isize) {
        self.scroll_lines(pages.saturating_mul(self.height.max(1) as isize));
    }
    /// Move by half pages, rounded up.
    pub fn scroll_half_pages(&mut self, pages: isize) {
        self.scroll_lines(pages.saturating_mul(self.height.max(1).div_ceil(2) as isize));
    }
    /// Move horizontally by terminal cells. Ignored while wrapping.
    pub fn scroll_horizontal(&mut self, cells: isize) {
        if self.key.is_some_and(|k| k.wrap) {
            return;
        }
        self.horizontal = self.horizontal.saturating_add_signed(cells);
        self.horizontal = self
            .horizontal
            .min(self.max_width.saturating_sub(self.content_width));
    }
    /// Move to the first displayed row.
    pub fn start(&mut self) {
        self.offset = 0;
    }
    /// Move to the final viewport.
    pub fn end(&mut self) {
        self.offset = self.screen.len();
        self.clamp();
    }
    /// Jump to the next hunk header.
    pub fn next_hunk(&mut self) {
        self.jump(true, false);
    }
    /// Jump to the previous hunk header.
    pub fn previous_hunk(&mut self) {
        self.jump(false, false);
    }
    /// Jump to the next file header.
    pub fn next_file(&mut self) {
        self.jump(true, true);
    }
    /// Jump to the previous file header.
    pub fn previous_file(&mut self) {
        self.jump(false, true);
    }
    /// Map an absolute displayed row to a numbered source line.
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
    /// Scroll to an available source line. Returns false when the patch omits it.
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
    fn clamp(&mut self) {
        self.offset = self
            .offset
            .min(self.screen.len().saturating_sub(self.height.max(1)));
    }
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
/// A borrowed diff widget. Configuration changes affect layout only when necessary.
#[derive(Debug, Clone, Copy)]
pub struct Diff<'a> {
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
    pub fn new(document: &'a DiffDocument) -> Self {
        Self {
            document,
            mode: ViewMode::Unified,
            theme: DiffTheme::default(),
            words: true,
            numbers: true,
            wrap: false,
            whitespace: false,
            tab: 4,
        }
    }
    /// Select unified or split presentation.
    pub fn mode(mut self, mode: ViewMode) -> Self {
        self.mode = mode;
        self
    }
    /// Override display styles without invalidating layout.
    pub fn theme(mut self, theme: DiffTheme) -> Self {
        self.theme = theme;
        self
    }
    /// Enable or disable inline emphasis without invalidating layout.
    pub fn word_highlights(mut self, enabled: bool) -> Self {
        self.words = enabled;
        self
    }
    /// Show or hide line numbers.
    pub fn line_numbers(mut self, visible: bool) -> Self {
        self.numbers = visible;
        self
    }
    /// Wrap long lines instead of scrolling horizontally.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
    /// Display space and tab markers.
    pub fn whitespace(mut self, visible: bool) -> Self {
        self.whitespace = visible;
        self
    }
    /// Set tab stops in cells. Zero is clamped to one.
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab = width.max(1);
        self
    }
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
        state.height = usize::from(area.height);
        if state.key == Some(key) {
            state.clamp();
            return;
        }
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
        let rebuild_rows = state.key.is_none_or(|previous| {
            previous.document != key.document
                || previous.mode != key.mode
                || previous.whitespace != key.whitespace
                || previous.tab != key.tab
        });
        if rebuild_rows {
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
                });
            }
        }
        state.old_sources.sort_unstable();
        state.new_sources.sort_unstable();
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
    fn pane_width(self, width: u16, header: bool) -> usize {
        if self.mode == ViewMode::Split && !header {
            usize::from(width.saturating_sub(1)) / 2
        } else {
            usize::from(width)
        }
    }
    fn gutter_width(self, digits: usize, header: bool) -> usize {
        if header {
            0
        } else if !self.numbers {
            2
        } else if self.mode == ViewMode::Unified {
            digits * 2 + 4
        } else {
            digits + 3
        }
    }
}
impl StatefulWidget for &Diff<'_> {
    type State = DiffState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut DiffState) {
        self.prepare(area, state);
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
                &screen.left,
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
                self.draw(
                    right,
                    &screen.right,
                    x.saturating_add(1),
                    py,
                    pane_width,
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
    #[allow(clippy::too_many_arguments)] // Geometry is explicit at the sole cell-writing boundary.
    fn draw(
        self,
        content: &Content,
        segment: &Range<usize>,
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
        if !header {
            let marker = match content.kind {
                LineKind::Context => ' ',
                LineKind::Insert => '+',
                LineKind::Delete => '-',
            };
            let number = |n: Option<usize>| {
                n.map_or_else(
                    || " ".repeat(state.digits),
                    |n| format!("{n:>w$}", w = state.digits),
                )
            };
            let prefix = if !self.numbers {
                format!("{marker} ")
            } else if self.mode == ViewMode::Unified {
                format!("{} {} {marker} ", number(content.old), number(content.new))
            } else {
                format!(
                    "{} {marker} ",
                    number(if side == Side::Old {
                        content.old
                    } else {
                        content.new
                    })
                )
            };
            buf.set_stringn(x, y, prefix, gutter, self.theme.gutter);
        }
        if width == 0 || segment.is_empty() {
            return;
        }
        let base = if self.wrap {
            content.glyphs[segment.start].column
        } else {
            state.horizontal
        };
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
            buf.set_stringn(
                x + (gutter + column) as u16,
                y,
                &glyph.text,
                glyph.width,
                style,
            );
        }
    }
}
fn empty() -> Content {
    Content {
        glyphs: Vec::new(),
        kind: LineKind::Context,
        old: None,
        new: None,
        width: 0,
    }
}
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
                    column,
                    width: 1,
                    emphasized,
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
                column,
                width,
                emphasized,
            });
            column += width;
        }
    }
    if !line.terminated && (line.old.is_some() || line.new.is_some()) {
        for g in " ⏎".graphemes(true) {
            let width = g.width();
            glyphs.push(Glyph {
                text: g.into(),
                column,
                width,
                emphasized: false,
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
