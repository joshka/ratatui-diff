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
    /// CRLF and final-newline differences participate in comparison. Replacement lines use bounded,
    /// monotonic similarity pairing for word highlights and split presentation. Oversized runs
    /// fall back to source order. Changed words separated only by whitespace share one
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
#[derive(Clone, Copy)]
enum AlignmentStep {
    Pair,
    SkipOld,
    SkipNew,
}

/// Pair one context-free replacement run; indexes refer to the original hunk slice.
///
/// Both refinement and split layout use this decision, including its bounded fallback.
pub(crate) fn replacement_pairs(lines: &[DiffLine]) -> Vec<(Option<usize>, Option<usize>)> {
    let deletes: Vec<_> = (0..lines.len())
        .filter(|&i| lines[i].kind == LineKind::Delete)
        .collect();
    let inserts: Vec<_> = (0..lines.len())
        .filter(|&i| lines[i].kind == LineKind::Insert)
        .collect();
    let source_order = || {
        (0..deletes.len().max(inserts.len()))
            .map(|i| (deletes.get(i).copied(), inserts.get(i).copied()))
            .collect()
    };

    // Bound preparation before tokenizing. The pair-work estimate also bounds all sorted
    // multiset intersections, independently of how similar the candidate lines are.
    if deletes.is_empty() || inserts.is_empty() {
        return source_order();
    }
    let bytes: usize = lines.iter().map(|line| line.text.len()).sum();
    let work = deletes
        .len()
        .saturating_mul(inserts.len())
        .saturating_mul(bytes);
    if lines.len() > 256 || bytes > 32768 || work > 1_048_576 {
        return source_order();
    }
    let grams: Vec<_> = lines.iter().map(|line| bigrams(&line.text)).collect();
    let width = inserts.len() + 1;
    let mut scores = vec![0u32; (deletes.len() + 1) * width];
    let mut choices = vec![AlignmentStep::SkipOld; scores.len()];
    // Solve suffixes so traceback starts at the earliest source lines. Equal-score decisions
    // prefer a pair, then an old-side gap: repeated lines therefore have stable early partners.
    for a in (0..deletes.len()).rev() {
        for b in (0..inserts.len()).rev() {
            let index = a * width + b;
            let down = scores[index + width];
            let right = scores[index + 1];
            let reward = similarity(&grams[deletes[a]], &grams[inserts[b]]).saturating_sub(500);
            let paired = scores[index + width + 1] + u32::from(reward);
            if reward > 0 && paired >= down.max(right) {
                scores[index] = paired;
                choices[index] = AlignmentStep::Pair;
            } else if down >= right {
                scores[index] = down;
                choices[index] = AlignmentStep::SkipOld;
            } else {
                scores[index] = right;
                choices[index] = AlignmentStep::SkipNew;
            }
        }
    }

    let mut pairs = Vec::new();
    let (mut a, mut b) = (0, 0);
    let (mut old_start, mut new_start) = (0, 0);
    while a < deletes.len() && b < inserts.len() {
        match choices[a * width + b] {
            AlignmentStep::Pair => {
                append_source_order(&mut pairs, &deletes[old_start..a], &inserts[new_start..b]);
                pairs.push((Some(deletes[a]), Some(inserts[b])));
                a += 1;
                b += 1;
                old_start = a;
                new_start = b;
            }
            AlignmentStep::SkipOld => a += 1,
            AlignmentStep::SkipNew => b += 1,
        }
    }
    // Similarity supplies anchors, not evidence that remaining lines are unrelated. Preserve
    // familiar source-order replacements inside each gap (including completely dissimilar runs).
    append_source_order(&mut pairs, &deletes[old_start..], &inserts[new_start..]);
    pairs
}

fn append_source_order(
    pairs: &mut Vec<(Option<usize>, Option<usize>)>,
    deletes: &[usize],
    inserts: &[usize],
) {
    for i in 0..deletes.len().max(inserts.len()) {
        pairs.push((deletes.get(i).copied(), inserts.get(i).copied()));
    }
}

fn bigrams(text: &str) -> Vec<(char, char)> {
    let mut previous = '\0';
    let mut grams = Vec::new();
    for ch in text.trim().chars() {
        grams.push((previous, ch));
        previous = ch;
    }
    grams.sort_unstable();
    grams
}

// Dice overlap retains Unicode scalar values and duplicate counts. Sorted bigrams avoid
// randomized hashes and expensive per-candidate edit-distance calculations.
fn similarity(left: &[(char, char)], right: &[(char, char)]) -> u16 {
    if left.is_empty() && right.is_empty() {
        return 1000;
    }
    let (mut a, mut b, mut common) = (0, 0, 0);
    while a < left.len() && b < right.len() {
        match left[a].cmp(&right[b]) {
            std::cmp::Ordering::Less => a += 1,
            std::cmp::Ordering::Greater => b += 1,
            std::cmp::Ordering::Equal => {
                common += 1;
                a += 1;
                b += 1;
            }
        }
    }
    (2000 * common / (left.len() + right.len())) as u16
}

/// Fill missing highlights within context-free replacement runs.
///
/// Use the same replacement pairing as split layout. Limit each run to 256 lines and each pair to a
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
        if end - start <= 256 {
            for (a, b) in replacement_pairs(&lines[start..end]) {
                let (Some(a), Some(b)) = (a, b) else {
                    continue;
                };
                let (a, b) = (start + a, start + b);
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
