//! Validated source documents, independent of rendering and comparison engines.

use std::error::Error;
use std::fmt;
use std::ops::Range;

/// A source side in a two-way diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Original source.
    Old,

    /// Modified source.
    New,
}

/// The role of a source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Unchanged on both sides.
    Context,

    /// Present only in the original.
    Delete,

    /// Present only in the modified source.
    Insert,
}

/// A logical source line; content excludes LF but retains any preceding CR.
///
/// Context lines have both source numbers, deletions only `old`, and insertions only `new`.
/// Constructing a line does not validate it; [`DiffDocument::new`] checks it against its hunk.
/// An unterminated line must be the last line on each side it belongs to, across all hunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Line role.
    pub kind: LineKind,

    /// Original line number, starting at one.
    pub old: Option<usize>,

    /// Modified line number, starting at one.
    pub new: Option<usize>,

    /// UTF-8 source content without LF. A trailing CR is retained for CRLF input.
    pub text: String,

    /// Whether this line ends with a newline in its source.
    pub terminated: bool,

    /// Changed byte ranges within [`text`](Self::text); `None` requests automatic refinement.
    ///
    /// Ranges must be ordered, non-overlapping, and on UTF-8 character boundaries. Empty ranges
    /// are accepted. `Some(vec![])` explicitly disables automatic highlights for this line.
    /// Ranges may bisect a grapheme; rendering emphasizes the whole intersecting grapheme.
    /// Automatic refinement may leave `None` for unpaired lines or large replacements.
    pub highlights: Option<Vec<Range<usize>>>,
}

impl DiffLine {
    /// Create a line from source text, preserving final-newline and CRLF differences.
    ///
    /// Removes one trailing LF and records it in [`terminated`](Self::terminated). A preceding
    /// CR remains in `text`. Embedded LF, numbering, and highlight validity are checked only when
    /// the line is passed to [`DiffDocument::new`].
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::{DiffLine, LineKind};
    ///
    /// let line = DiffLine::new(LineKind::Insert, None, Some(1), "hello\r\n");
    /// assert_eq!(line.text, "hello\r");
    /// assert!(line.terminated);
    /// ```
    pub fn new(kind: LineKind, old: Option<usize>, new: Option<usize>, text: &str) -> Self {
        Self {
            kind,
            old,
            new,
            text: text.strip_suffix('\n').unwrap_or(text).to_owned(),
            terminated: text.ends_with('\n'),
            highlights: None,
        }
    }
}

/// A group of changes with their available context.
///
/// Nonempty ranges use one-based source numbers: `3..5` contains lines 3 and 4. An empty
/// range identifies the preceding line, so `0..0` represents insertion before the first line.
/// Each side's range length must equal the number of lines carrying a number on that side.
/// Hunks in a file must be ordered and non-overlapping on both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// Original line range, start-inclusive and end-exclusive. Empty sides may start at zero.
    pub old: Range<usize>,

    /// Modified line range, start-inclusive and end-exclusive. Empty sides may start at zero.
    pub new: Range<usize>,

    /// Context, deleted, and inserted lines in display order; each side is numbered sequentially.
    pub lines: Vec<DiffLine>,
}

/// File-level information, including metadata-only changes.
///
/// At least one path is required. Paths are display labels; the library does not open files.
/// A binary file must have no text hunks. An empty hunk list is also valid for metadata-only
/// changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    /// Original path, absent for additions.
    pub old_path: Option<String>,

    /// Modified path, absent for deletions.
    pub new_path: Option<String>,

    /// Extended headers displayed below the file label, in supplied order.
    pub metadata: Vec<String>,

    /// Whether this is an opaque binary change.
    pub binary: bool,

    /// Available text hunks.
    pub hunks: Vec<Hunk>,
}

/// A validated, prepared diff. Construct once and borrow it for each frame.
///
/// Use [`from_text`](Self::from_text) for two source strings, [`parse`](Self::parse) for patches,
/// or [`new`](Self::new) for structured input with optional caller-supplied highlights.
/// Files are immutable after construction and may be shared by independently navigated widgets.
/// Cloning preserves document identity, so rendering a clone with an existing state retains its
/// viewport and caches. Constructing a new document resets that state on its next render.
#[derive(Debug, Clone)]
pub struct DiffDocument {
    pub(crate) files: Vec<DiffFile>,

    // Clones share this identity because their validated content cannot change.
    pub(crate) id: u64,
}

/// Invalid input with a byte position when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffError {
    /// Zero-based byte position in the original patch input, when available.
    ///
    /// A parser error may point to the start of its file chunk. Errors from structured validation
    /// have no position, including validation failures encountered while parsing a patch.
    pub offset: Option<usize>,

    /// Human-readable cause. Display includes `at byte N: ` when an offset is available.
    pub message: String,
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "at byte {offset}: ")?;
        }
        f.write_str(&self.message)
    }
}

impl Error for DiffError {}
impl DiffError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self {
            offset: None,
            message: message.into(),
        }
    }
}

impl DiffDocument {
    /// Validate structured input and prepare missing word highlights.
    ///
    /// Preserves file order and caller-supplied highlights, including explicit empty lists.
    /// Missing highlights are computed for paired replacement lines within the refinement limits.
    /// Empty documents and metadata-only files are valid.
    ///
    /// # Errors
    ///
    /// Returns [`DiffError`] without an offset if a file has no path, a binary file has text hunks,
    /// or any of these source constraints fail:
    ///
    /// - Hunk ranges are forward, ordered, and non-overlapping on each side.
    /// - Line kinds, one-based numbers, and hunk counts agree, with sequential numbers per side.
    /// - No source line follows an unterminated line on the same side, even in a later hunk.
    /// - Line text contains no LF; highlights are ordered, non-overlapping UTF-8 byte ranges.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::{DiffDocument, DiffFile, DiffLine, Hunk, LineKind};
    ///
    /// let document = DiffDocument::new(vec![DiffFile {
    ///     old_path: None,
    ///     new_path: Some("greeting.txt".into()),
    ///     metadata: vec![],
    ///     binary: false,
    ///     hunks: vec![Hunk {
    ///         old: 0..0,
    ///         new: 1..2,
    ///         lines: vec![DiffLine::new(LineKind::Insert, None, Some(1), "hello\n")],
    ///     }],
    /// }])?;
    /// assert_eq!(document.files()[0].hunks[0].new, 1..2);
    /// # Ok::<(), ratatui_diff::DiffError>(())
    /// ```
    pub fn new(mut files: Vec<DiffFile>) -> Result<Self, DiffError> {
        for file in &mut files {
            if file.old_path.is_none() && file.new_path.is_none() {
                return Err(DiffError::invalid("file has no source path"));
            }
            if file.binary && !file.hunks.is_empty() {
                return Err(DiffError::invalid("binary file contains text hunks"));
            }

            // Validate each side as a forward-only source cursor. An unterminated line closes
            // that side for the entire file, even when the next hunk omits intervening context.
            let mut old_finished = false;
            let mut new_finished = false;
            let mut previous = None;
            for hunk in &mut file.hunks {
                if let Some((old_end, new_end)) = previous
                    && (hunk.old.start < old_end || hunk.new.start < new_end)
                {
                    return Err(DiffError::invalid("overlapping or unordered hunks"));
                }
                previous = Some((hunk.old.end, hunk.new.end));
                if hunk.old.end < hunk.old.start || hunk.new.end < hunk.new.start {
                    return Err(DiffError::invalid("reversed hunk range"));
                }

                // Hunk bounds constrain cursor advances before addition, preventing overflow.
                let (mut old, mut new) = (hunk.old.start, hunk.new.start);
                for line in &hunk.lines {
                    if (old_finished && line.old.is_some()) || (new_finished && line.new.is_some())
                    {
                        return Err(DiffError::invalid(
                            "source line follows an unterminated final line",
                        ));
                    }
                    old_finished |= line.old.is_some() && !line.terminated;
                    new_finished |= line.new.is_some() && !line.terminated;
                    let expected = match line.kind {
                        LineKind::Context => (Some(old), Some(new)),
                        LineKind::Delete => (Some(old), None),
                        LineKind::Insert => (None, Some(new)),
                    };
                    if (line.old, line.new) != expected
                        || line.old == Some(0)
                        || line.new == Some(0)
                    {
                        return Err(DiffError::invalid("source line numbers do not match hunk"));
                    }
                    if (line.old.is_some() && old >= hunk.old.end)
                        || (line.new.is_some() && new >= hunk.new.end)
                    {
                        return Err(DiffError::invalid("line exceeds hunk range"));
                    }
                    old += usize::from(line.old.is_some());
                    new += usize::from(line.new.is_some());
                    if line.text.contains('\n') {
                        return Err(DiffError::invalid("line contains newline"));
                    }

                    // Highlight ranges must be ordered UTF-8 slices of this line, never overlap.
                    let mut end = 0;
                    for r in line.highlights.iter().flatten() {
                        if r.start < end
                            || r.end < r.start
                            || r.end > line.text.len()
                            || !line.text.is_char_boundary(r.start)
                            || !line.text.is_char_boundary(r.end)
                        {
                            return Err(DiffError::invalid("invalid highlight range"));
                        }
                        end = r.end;
                    }
                }

                // Consuming exactly the advertised ranges catches truncated structured hunks.
                if old != hunk.old.end || new != hunk.new.end {
                    return Err(DiffError::invalid("hunk counts do not match lines"));
                }
                crate::compare::refine(&mut hunk.lines);
            }
        }
        Ok(Self {
            files,
            id: next_id(),
        })
    }

    /// Inspect files in input order, including prepared highlights and preserved metadata.
    pub fn files(&self) -> &[DiffFile] {
        &self.files
    }
}

/// Allocate a cache identity independent of allocation addresses.
///
/// Relaxed ordering is sufficient: the counter distinguishes documents and does not publish their
/// contents across threads. Cloning a document copies its identity instead of calling this helper.
pub(crate) fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
