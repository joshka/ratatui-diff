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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Line role.
    pub kind: LineKind,

    /// Original line number, starting at one.
    pub old: Option<usize>,

    /// Modified line number, starting at one.
    pub new: Option<usize>,

    /// UTF-8 source content, without a newline.
    pub text: String,

    /// Whether this line ends with a newline in its source.
    pub terminated: bool,

    /// Changed byte ranges; `None` requests automatic refinement.
    pub highlights: Option<Vec<Range<usize>>>,
}
impl DiffLine {
    /// Create a line from source text, preserving final-newline and CRLF differences.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// Original line range, start-inclusive and end-exclusive. Empty sides may start at zero.
    pub old: Range<usize>,

    /// Modified line range, start-inclusive and end-exclusive. Empty sides may start at zero.
    pub new: Range<usize>,

    /// Source lines in unified order.
    pub lines: Vec<DiffLine>,
}
/// File-level information, including metadata-only changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    /// Original path, absent for additions.
    pub old_path: Option<String>,

    /// Modified path, absent for deletions.
    pub new_path: Option<String>,

    /// Extended headers preserved for display.
    pub metadata: Vec<String>,

    /// Whether this is an opaque binary change.
    pub binary: bool,

    /// Available text hunks.
    pub hunks: Vec<Hunk>,
}
/// A validated, prepared diff. Construct once and borrow it for each frame.
#[derive(Debug, Clone)]
pub struct DiffDocument {
    pub(crate) files: Vec<DiffFile>,
    pub(crate) id: u64,
}
/// Invalid input with a byte position when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffError {
    /// Position in patch input; absent for structured-input errors.
    pub offset: Option<usize>,

    /// Human-readable cause.
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
    /// Ranges must agree with the lines, numbers must be sequential, and highlights must
    /// be ordered, non-overlapping UTF-8 byte ranges. Empty documents are valid.
    pub fn new(mut files: Vec<DiffFile>) -> Result<Self, DiffError> {
        for file in &mut files {
            if file.old_path.is_none() && file.new_path.is_none() {
                return Err(DiffError::invalid("file has no source path"));
            }
            if file.binary && !file.hunks.is_empty() {
                return Err(DiffError::invalid("binary file contains text hunks"));
            }
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
    /// Inspect the validated source files.
    pub fn files(&self) -> &[DiffFile] {
        &self.files
    }
}

// Identity prevents stale cached layouts when an allocation address is reused.
pub(crate) fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
