//! Comparison stays outside frame rendering.
use std::ops::Range;

use similar::{ChangeTag, TextDiff};

use crate::{DiffDocument, DiffFile, DiffLine, Hunk, LineKind};

impl DiffDocument {
    /// Compare two UTF-8 texts with three context lines and generic source labels.
    ///
    /// Equivalent to [`compare`](Self::compare) with `context = 3`.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::DiffDocument;
    ///
    /// let document = DiffDocument::from_text("hello\n", "hello world\n");
    /// assert_eq!(document.files().len(), 1);
    /// assert_eq!(document.files()[0].hunks.len(), 1);
    /// ```
    pub fn from_text(old: &str, new: &str) -> Self {
        Self::compare(old, new, 3)
    }
    /// Compare texts with a caller-selected number of context lines.
    ///
    /// Returns one file labelled `old` and `new`, even for identical or empty inputs. Identical
    /// inputs have no hunks. `context` controls unchanged lines around edits; zero shows only
    /// changed lines, and nearby edits may share a hunk.
    ///
    /// Use `usize::MAX` to retain all source lines when the inputs differ. Context is clamped to
    /// the larger input's line count before grouping, so this does not allocate artificial lines.
    /// Retained context remains available for search and source-text extraction. Smaller context
    /// counts discard omitted lines; the document cannot later recover them. Identical inputs
    /// still have no hunks, even with `usize::MAX`.
    ///
    /// CRLF and final-newline differences participate in comparison. Replacement lines are paired
    /// in source order for word highlights. Changed words separated only by whitespace share one
    /// highlight range; large pairs fall back to whole-line styling.
    /// Computation is synchronous; prepare large inputs outside the UI event loop.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::DiffDocument;
    ///
    /// let document = DiffDocument::compare("same\nbefore\n", "same\nafter\n", 0);
    /// assert_eq!(document.files()[0].hunks[0].old, 2..3);
    /// ```
    pub fn compare(old: &str, new: &str, context: usize) -> Self {
        let diff = TextDiff::from_lines(old, new);
        let mut hunks = Vec::new();
        let context = context.min(old.lines().count().max(new.lines().count()));
        for group in diff.grouped_ops(context) {
            let mut lines = Vec::new();
            let first = &group[0];
            let last = &group[group.len() - 1];
            let old_start = first.old_range().start;
            let old_end = last.old_range().end;
            let new_start = first.new_range().start;
            let new_end = last.new_range().end;
            // Empty sides identify the preceding line, as unified hunk headers do.
            let old_range = if old_start == old_end {
                old_start..old_start
            } else {
                old_start + 1..old_end + 1
            };
            let new_range = if new_start == new_end {
                new_start..new_start
            } else {
                new_start + 1..new_end + 1
            };
            for op in &group {
                for c in diff.iter_changes(op) {
                    let kind = match c.tag() {
                        ChangeTag::Equal => LineKind::Context,
                        ChangeTag::Delete => LineKind::Delete,
                        ChangeTag::Insert => LineKind::Insert,
                    };
                    lines.push(DiffLine::new(
                        kind,
                        c.old_index().map(|n| n + 1),
                        c.new_index().map(|n| n + 1),
                        c.value(),
                    ));
                }
            }
            refine(&mut lines);
            hunks.push(Hunk {
                old: old_range,
                new: new_range,
                lines,
            });
        }
        Self {
            files: vec![DiffFile {
                old_path: Some("old".into()),
                new_path: Some("new".into()),
                metadata: Vec::new(),
                binary: false,
                hunks,
            }],
            id: crate::model::next_id(),
        }
    }
}
/// Fill missing highlights for deletion/insertion pairs separated by context lines.
///
/// Pair in source order, matching split layout. Limit each run to 256 lines and each pair to a
/// combined 8 KiB / 2,048 whitespace-delimited words. Unmatched or over-limit lines retain their
/// existing highlights; `None` renders with whole-line styling. Explicit ranges, including empty
/// lists, remain authoritative. These limits bound refinement, not whole-document comparison.
pub(crate) fn refine(lines: &mut [DiffLine]) {
    let mut start = 0;
    while start < lines.len() {
        if lines[start].kind == LineKind::Context {
            start += 1;
            continue;
        }
        let end = lines[start..]
            .iter()
            .position(|l| l.kind == LineKind::Context)
            .map_or(lines.len(), |n| start + n);
        let deletes: Vec<_> = (start..end)
            .filter(|&n| lines[n].kind == LineKind::Delete)
            .collect();
        let inserts: Vec<_> = (start..end)
            .filter(|&n| lines[n].kind == LineKind::Insert)
            .collect();
        if end - start <= 256 {
            for (&a, &b) in deletes.iter().zip(&inserts) {
                let old = &lines[a].text;
                let new = &lines[b].text;
                if old.len() + new.len() > 8192
                    || old.split_whitespace().count() + new.split_whitespace().count() > 2048
                {
                    continue;
                }
                let diff = TextDiff::from_words(old, new);
                let (mut x, mut y) = (0, 0);
                let (mut left, mut right) = (Vec::new(), Vec::new());
                for c in diff.iter_all_changes() {
                    let len = c.value().len();
                    match c.tag() {
                        ChangeTag::Equal => {
                            x += len;
                            y += len;
                        }
                        ChangeTag::Delete => {
                            push_highlight(&mut left, x..x + len, old);
                            x += len;
                        }
                        ChangeTag::Insert => {
                            push_highlight(&mut right, y..y + len, new);
                            y += len;
                        }
                    }
                }
                if lines[a].highlights.is_none() {
                    lines[a].highlights = Some(left);
                }
                if lines[b].highlights.is_none() {
                    lines[b].highlights = Some(right);
                }
            }
        }
        start = end;
    }
}

// Changed words separated only by whitespace read as one phrase. Keep the outer boundaries exact
// and leave caller-supplied ranges untouched; only automatic refinement calls this helper.
fn push_highlight(ranges: &mut Vec<Range<usize>>, range: Range<usize>, text: &str) {
    if let Some(previous) = ranges.last_mut()
        && text[previous.end..range.start]
            .chars()
            .all(char::is_whitespace)
    {
        previous.end = range.end;
    } else {
        ranges.push(range);
    }
}
