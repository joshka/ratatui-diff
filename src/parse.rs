//! The Diffy adapter preserves payload endings while normalizing header delimiters.
use diffy::patch_set::{FileOperation, ParseOptions, PatchSet};

use crate::{DiffDocument, DiffError, DiffFile, DiffLine, Hunk, LineKind};

impl DiffDocument {
    /// Parse standard unified patches or Git's two-way patch output.
    ///
    /// Each `diff --git` section is read as Git's extended format; other sections are read as plain
    /// unified diffs, which covers Mercurial, Subversion, and `diff -u` output. Commit or mail
    /// headers before the first Git section are skipped, so `git show`, `git format-patch`, and
    /// `jj show --git` output parse directly. Empty input is an empty document. Context and
    /// combined merge diffs are rejected. Binary changes are summaries; this library never applies
    /// patches or decompresses payloads.
    ///
    /// Each file keeps its own extended or tool headers as metadata. Git's default `a/` and `b/`
    /// prefixes are removed from paths; plain unified paths are kept as written unless both sides
    /// carry those prefixes for the same file. CRLF payload endings and missing final newlines are
    /// preserved; only lines present in the patch are available for navigation. Parsing also
    /// validates the model and prepares missing highlights via [`Self::new`].
    ///
    /// # Errors
    ///
    /// Returns [`DiffError`] for malformed patches, hunks with more lines than their header counts,
    /// unsupported context or combined diffs, or invalid source structure. Parser positions refer
    /// to bytes in the original input, before header normalization. Validation errors may have no
    /// offset.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_diff::DiffDocument;
    ///
    /// let patch = "--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n";
    /// let document = DiffDocument::parse(patch)?;
    /// assert_eq!(document.files()[0].hunks[0].lines.len(), 2);
    /// assert!(DiffDocument::parse("diff --cc file\n").is_err());
    /// # Ok::<(), ratatui_diff::DiffError>(())
    /// ```
    pub fn parse(input: &str) -> Result<Self, DiffError> {
        if input.trim().is_empty() {
            return Self::new(Vec::new());
        }
        let mut files = Vec::new();
        let mut offset = 0;
        let mut chunks = Vec::new();
        let mut chunk_start = 0;
        let mut previous = (0, "");
        // Text before the first Git section is a preamble (commit or mail headers) unless it
        // holds a hunk of its own.
        let mut preamble = true;
        for line in input.split_inclusive('\n') {
            if line.starts_with("diff --cc ")
                || line.starts_with("diff --combined ")
                || line.starts_with("@@@")
            {
                return Err(DiffError::at(
                    Some(offset),
                    "combined merge diffs are unsupported",
                ));
            }
            if previous.1.starts_with("*** ") && line.starts_with("--- ") {
                return Err(DiffError::at(
                    Some(previous.0),
                    "context diffs are unsupported",
                ));
            }
            if line.starts_with("@@ ") {
                preamble = false;
            }
            if line.starts_with("diff --git ") {
                if offset > chunk_start && !preamble {
                    chunks.push((chunk_start, &input[chunk_start..offset]));
                }
                chunk_start = offset;
                preamble = false;
            }
            previous = (offset, line);
            offset += line.len();
        }
        chunks.push((chunk_start, &input[chunk_start..]));
        for (base, chunk) in chunks {
            for (base, segment) in file_segments(base, chunk)? {
                parse_file(base, segment, &mut files)?;
            }
        }
        Self::new(files)
    }
}

/// Split a chunk into one segment per file, rejecting hunks with more lines than their header.
///
/// A Git chunk is already a single file. Plain unified chunks may hold several files; each new file
/// begins at the first line after a hunk that is not another hunk, so its leading headers become
/// its own metadata. Trailing text after the last file belongs to that file.
fn file_segments(base: usize, chunk: &str) -> Result<Vec<(usize, &str)>, DiffError> {
    let git = chunk.starts_with("diff --git ");
    let mut starts = vec![0];
    let mut offset = 0;
    let mut remaining: Option<(usize, usize)> = None;
    let mut hunk_ended = false;
    let mut seen_hunk = false;
    for line in chunk.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        if let Some((old, new)) = &mut remaining {
            match body.chars().next() {
                Some('\\') => {}
                Some(' ') | None => (*old, *new) = (old.saturating_sub(1), new.saturating_sub(1)),
                Some('-') => *old = old.saturating_sub(1),
                Some('+') => *new = new.saturating_sub(1),
                // Malformed bodies are reported by the patch parser.
                Some(_) => remaining = None,
            }
            if remaining.is_some_and(|counts| counts == (0, 0)) {
                remaining = None;
                hunk_ended = true;
            }
            offset += line.len();
            continue;
        }
        if hunk_ended && !body.starts_with('\\') {
            hunk_ended = false;
            let extra = body.starts_with([' ', '+', '-'])
                && !body.starts_with("--- ")
                && !matches!(body, "-- " | "--");
            if extra {
                return Err(DiffError::at(
                    Some(base + offset),
                    "hunk has more lines than its header counts",
                ));
            }
        }
        if body.starts_with("@@ ") {
            remaining = hunk_counts(body);
            seen_hunk = true;
        } else if !git && seen_hunk && !body.starts_with('\\') {
            starts.push(offset);
            seen_hunk = false;
        }
        offset += line.len();
    }
    // Text after the last plain file, such as a mail signature, is that file's trailer.
    let tail = &chunk[starts[starts.len() - 1]..];
    let has_file = tail
        .split_inclusive('\n')
        .any(|line| line.starts_with("--- ") || line.starts_with("@@ "));
    if starts.len() > 1 && !has_file {
        starts.pop();
    }
    let ends = starts.iter().skip(1).copied().chain([chunk.len()]);
    let segments = starts
        .iter()
        .zip(ends)
        .map(|(&start, end)| (base + start, &chunk[start..end]))
        .collect();
    Ok(segments)
}

/// Read the old and new line counts from a unified hunk header; omitted counts are one.
fn hunk_counts(header: &str) -> Option<(usize, usize)> {
    let mut ranges = header.strip_prefix("@@ ")?.split(' ');
    let count = |range: &str, sign: char| -> Option<usize> {
        let range = range.strip_prefix(sign)?;
        match range.split_once(',') {
            Some((_, count)) => count.parse().ok(),
            None => range.parse::<usize>().ok().map(|_| 1),
        }
    };
    let old = count(ranges.next()?, '-')?;
    let new = count(ranges.next()?, '+')?;
    Some((old, new))
}

/// Parse one file's segment, mapping parser offsets back to the original input.
fn parse_file(base: usize, segment: &str, files: &mut Vec<DiffFile>) -> Result<(), DiffError> {
    let mut normalized = String::new();
    let mut metadata = Vec::new();
    let mut binary = false;
    let mut in_hunk = false;
    let mut byte_map = Vec::new();
    let mut patch_start = None;
    let mut source_offset = base;
    for line in segment.split_inclusive('\n') {
        if line.starts_with("@@ ") {
            in_hunk = true;
        }
        if !in_hunk && line.starts_with("--- ") {
            patch_start.get_or_insert(normalized.len());
        }
        if !in_hunk && !binary && !line.starts_with("--- ") && !line.starts_with("+++ ") {
            metadata.push(line.trim_end_matches(['\r', '\n']).to_owned());
        }
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            binary = true;
        }
        let text = if !in_hunk {
            line.strip_suffix("\r\n")
                .map(|s| format!("{s}\n"))
                .unwrap_or_else(|| line.to_owned())
        } else {
            line.to_owned()
        };
        byte_map.push((normalized.len(), source_offset));
        normalized.push_str(&text);
        source_offset += line.len();
    }
    let git = normalized.starts_with("diff --git ");
    let options = if git {
        ParseOptions::gitdiff()
    } else {
        ParseOptions::unidiff()
    };
    // Git's default `a/` and `b/` prefixes are not part of repository paths.
    let prefixed = git && {
        let header = normalized.lines().next().unwrap_or_default();
        let header = header.strip_prefix("diff --git ").unwrap_or_default();
        header.trim_start_matches('"').starts_with("a/")
            && (header.contains(" b/") || header.contains(" \"b/"))
    };
    for parsed in PatchSet::parse(&normalized, options) {
        let patch = parsed.map_err(|e| {
            let message = e.to_string();
            // Diffy positions count from the start of the file's `---` header.
            let (n, message) = match message.rsplit_once("at byte ") {
                Some((_, rest)) => match rest.split_once(": ") {
                    Some((n, rest)) => (n.parse::<usize>().ok(), rest.to_owned()),
                    None => (None, message.clone()),
                },
                None => (None, message.clone()),
            };
            let offset = n
                .zip(patch_start)
                .map(|(n, start)| {
                    let n = start + n;
                    let line = byte_map
                        .partition_point(|&(normalized, _)| normalized <= n)
                        .saturating_sub(1);
                    let (normalized, original) = byte_map[line];
                    original + n.saturating_sub(normalized)
                })
                .unwrap_or(base);
            DiffError::at(Some(offset), message)
        })?;
        let strip = |path: &str, prefix: &str| -> String {
            path.strip_prefix(prefix).unwrap_or(path).to_owned()
        };
        let (old_path, new_path) = match patch.operation() {
            FileOperation::Delete(p) if prefixed => (Some(strip(p, "a/")), None),
            FileOperation::Create(p) if prefixed => (None, Some(strip(p, "b/"))),
            FileOperation::Delete(p) => (Some(p.to_string()), None),
            FileOperation::Create(p) => (None, Some(p.to_string())),
            FileOperation::Modify { original, modified } if prefixed => {
                (Some(strip(original, "a/")), Some(strip(modified, "b/")))
            }
            FileOperation::Modify { original, modified } => {
                // Plain unified diffs from Git-style tools name the same file on both sides.
                match (original.strip_prefix("a/"), modified.strip_prefix("b/")) {
                    (Some(old), Some(new)) if old == new => (Some(old.into()), Some(new.into())),
                    _ => (Some(original.to_string()), Some(modified.to_string())),
                }
            }
            FileOperation::Rename { from, to } | FileOperation::Copy { from, to } => {
                (Some(from.to_string()), Some(to.to_string()))
            }
        };
        let mut hunks = Vec::new();
        if let Some(text) = patch.patch().as_text() {
            for h in text.hunks() {
                let (mut old, mut new) = (h.old_range().start(), h.new_range().start());
                let mut lines = Vec::new();
                for line in h.lines() {
                    let (kind, content, a, b) = match line {
                        diffy::Line::Context(s) => (LineKind::Context, *s, Some(old), Some(new)),
                        diffy::Line::Delete(s) => (LineKind::Delete, *s, Some(old), None),
                        diffy::Line::Insert(s) => (LineKind::Insert, *s, None, Some(new)),
                    };
                    lines.push(DiffLine::new(kind, a, b, content));
                    old += usize::from(a.is_some());
                    new += usize::from(b.is_some());
                }
                let heading = h
                    .function_context()
                    .map(|heading| heading.trim_end_matches(['\r', '\n']))
                    .filter(|heading| !heading.is_empty())
                    .map(str::to_owned);
                hunks.push(Hunk {
                    old: h.old_range().start()..old,
                    new: h.new_range().start()..new,
                    heading,
                    lines,
                });
            }
        }
        files.push(DiffFile {
            old_path,
            new_path,
            metadata: metadata.clone(),
            binary: binary || patch.patch().is_binary(),
            hunks,
        });
    }
    Ok(())
}
