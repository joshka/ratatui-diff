//! Source-text extraction shared by keyboard and pointer selection.

use unicode_segmentation::UnicodeSegmentation;

use crate::{DiffDocument, DiffLine, Side, SourcePosition};

/// A caret boundary in a numbered source line.
///
/// `byte` is a grapheme boundary in the retained text, or `text.len() + 1` to include LF on a
/// terminated line. CR remains part of the text. Coordinates belong to one immutable document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceBoundary {
    /// File, side, and one-based line number.
    pub position: SourcePosition,

    /// End-exclusive source byte boundary, including the optional LF position.
    pub byte: usize,
}

/// An anchored selection within one file and source side.
///
/// Anchor remains fixed when focus moves. Ordering is by `(line, byte)`, independently of display
/// order, wrapping, or split alignment. Empty selections are valid carets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSelection {
    /// Fixed source boundary.
    pub anchor: SourceBoundary,

    /// Moving source boundary.
    pub focus: SourceBoundary,
}

impl SourceSelection {
    /// Return start/end boundaries in source order, or `None` across file/side boundaries.
    pub fn ordered(self) -> Option<(SourceBoundary, SourceBoundary)> {
        if self.anchor.position.file != self.focus.position.file
            || self.anchor.position.side != self.focus.position.side
        {
            return None;
        }
        let key = |point: SourceBoundary| (point.position.line, point.byte);
        Some(if key(self.anchor) <= key(self.focus) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        })
    }

    /// Extract original source, preserving line endings and excluding all display notation.
    ///
    /// Returns `None` for invalid grapheme boundaries, missing lines, or file/side crossings.
    /// No ellipsis, padding, or replacement text is inserted for missing patch context.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::{DiffDocument, Side, SourceBoundary, SourcePosition, SourceSelection};
    ///
    /// let document = DiffDocument::from_text("", "hello\n");
    /// let anchor = SourceBoundary {
    ///     position: SourcePosition {
    ///         file: 0,
    ///         side: Side::New,
    ///         line: 1,
    ///     },
    ///     byte: 0,
    /// };
    /// let focus = SourceBoundary { byte: 6, ..anchor };
    /// let selected = SourceSelection { anchor, focus }.text(&document);
    /// assert_eq!(selected.as_deref(), Some("hello\n"));
    /// ```
    pub fn text(self, document: &DiffDocument) -> Option<String> {
        let (start, end) = self.ordered()?;
        extract(document, start.position, start.byte, end.position, end.byte)
    }

    pub(crate) fn intersects(
        self,
        position: SourcePosition,
        bytes: &std::ops::Range<usize>,
    ) -> bool {
        let Some((start, end)) = self.ordered() else {
            return false;
        };
        position.file == start.position.file
            && position.side == start.position.side
            && (position.line, bytes.end) > (start.position.line, start.byte)
            && (position.line, bytes.start) < (end.position.line, end.byte)
    }
}

/// Find a numbered line without manufacturing omitted patch context.
pub(crate) fn source_line(document: &DiffDocument, position: SourcePosition) -> Option<&DiffLine> {
    document
        .files()
        .get(position.file)?
        .hunks
        .iter()
        .find_map(|hunk| {
            hunk.lines.iter().find(|line| match position.side {
                Side::Old => line.old == Some(position.line),
                Side::New => line.new == Some(position.line),
            })
        })
}

/// Selection boundaries follow graphemes; LF occupies one additional byte after terminated text.
pub(crate) fn boundary(line: &DiffLine, byte: usize) -> bool {
    byte == line.text.len()
        || (line.terminated && byte == line.text.len() + 1)
        || line
            .text
            .grapheme_indices(true)
            .any(|(start, _)| start == byte)
}

pub(crate) fn extract(
    document: &DiffDocument,
    start: SourcePosition,
    start_byte: usize,
    end: SourcePosition,
    end_byte: usize,
) -> Option<String> {
    if start.file != end.file || start.side != end.side || start.line > end.line {
        return None;
    }
    let first = source_line(document, start)?;
    let last = source_line(document, end)?;
    if !boundary(first, start_byte) || !boundary(last, end_byte) {
        return None;
    }
    if start.line == end.line && start_byte > end_byte {
        return None;
    }
    let mut selected = String::new();
    let mut expected = start.line;
    for line in document
        .files()
        .get(start.file)?
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
    {
        let number = match start.side {
            Side::Old => line.old,
            Side::New => line.new,
        };
        let Some(number) = number.filter(|&number| number >= start.line && number <= end.line)
        else {
            continue;
        };
        if number != expected {
            return None;
        }
        let from = if number == start.line { start_byte } else { 0 };
        let to = if number == end.line {
            end_byte
        } else {
            line.text.len() + usize::from(line.terminated)
        };
        selected.push_str(
            line.text
                .get(from.min(line.text.len())..to.min(line.text.len()))?,
        );
        if to > line.text.len() && from <= line.text.len() {
            selected.push('\n');
        }
        if number == end.line {
            return Some(selected);
        }
        expected = expected.checked_add(1)?;
    }
    None
}

/// Source movement for a host's keyboard bindings, independent of wrapped display rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMotion {
    /// Advance one whole grapheme, then LF, then the next available adjacent source line.
    Next,
    /// Move backward through the same boundaries.
    Previous,
    /// Move to the next adjacent source line, clamping byte to a preceding grapheme boundary.
    NextLine,
    /// Move to the previous adjacent source line with the same clamping rule.
    PreviousLine,
    /// Move to the beginning of the current source line.
    LineStart,
    /// Move to the end of source text, before its LF.
    LineEnd,
}

impl DiffDocument {
    pub(crate) fn move_boundary(
        &self,
        point: SourceBoundary,
        motion: SelectionMotion,
    ) -> Option<SourceBoundary> {
        let line = source_line(self, point.position)?;
        if !boundary(line, point.byte) {
            return None;
        }
        let end = line.text.len() + usize::from(line.terminated);
        let byte = match motion {
            SelectionMotion::NextLine | SelectionMotion::PreviousLine => {
                let number = if motion == SelectionMotion::NextLine {
                    point.position.line.checked_add(1)?
                } else {
                    point.position.line.checked_sub(1)?
                };
                let position = SourcePosition {
                    line: number,
                    ..point.position
                };
                let adjacent = source_line(self, position)?;
                let byte = adjacent
                    .text
                    .grapheme_indices(true)
                    .map(|(byte, _)| byte)
                    .chain([adjacent.text.len()])
                    .filter(|&byte| byte <= point.byte)
                    .max()
                    .unwrap_or(0);
                return Some(SourceBoundary { position, byte });
            }
            SelectionMotion::LineStart => 0,
            SelectionMotion::LineEnd => line.text.len(),
            SelectionMotion::Next if point.byte < end => line
                .text
                .grapheme_indices(true)
                .map(|(byte, _)| byte)
                .chain([line.text.len(), end])
                .find(|&byte| byte > point.byte)?,
            SelectionMotion::Previous if point.byte > 0 => line
                .text
                .grapheme_indices(true)
                .map(|(byte, _)| byte)
                .chain([line.text.len()])
                .filter(|&byte| byte < point.byte)
                .max()?,
            SelectionMotion::Next | SelectionMotion::Previous => {
                let number = if motion == SelectionMotion::Next {
                    point.position.line.checked_add(1)?
                } else {
                    point.position.line.checked_sub(1)?
                };
                let position = SourcePosition {
                    line: number,
                    ..point.position
                };
                let adjacent = source_line(self, position)?;
                return Some(SourceBoundary {
                    position,
                    byte: if motion == SelectionMotion::Next {
                        0
                    } else {
                        adjacent.text.len() + usize::from(adjacent.terminated)
                    },
                });
            }
        };
        Some(SourceBoundary { byte, ..point })
    }
}

impl crate::SourceRange {
    /// Convert a hit grapheme's start into a selection caret boundary.
    pub fn start_boundary(&self) -> SourceBoundary {
        SourceBoundary {
            position: self.position,
            byte: self.bytes.start,
        }
    }

    /// Convert a hit grapheme's end into a selection caret boundary.
    pub fn end_boundary(&self) -> SourceBoundary {
        SourceBoundary {
            position: self.position,
            byte: self.bytes.end,
        }
    }
}
