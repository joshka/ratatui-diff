//! The Diffy adapter preserves payload endings while normalizing header delimiters.
use diffy::patch_set::{FileOperation, ParseOptions, PatchSet};

use crate::{DiffDocument, DiffError, DiffFile, DiffLine, Hunk, LineKind};

impl DiffDocument {
    /// Parse standard unified patches or Git's two-way patch output.
    ///
    /// Empty input is an empty document. Combined merge diffs are rejected. Binary
    /// changes are summaries; this library never applies patches or decompresses payloads.
    ///
    /// Extended headers and paths are retained for display. CRLF payload endings and missing final
    /// newlines are preserved; only lines present in the patch are available for navigation.
    /// Parsing also validates the model and prepares missing highlights via [`Self::new`].
    ///
    /// # Errors
    ///
    /// Returns [`DiffError`] for malformed patches, unsupported combined merge diffs, or invalid
    /// source structure. Parser positions refer to bytes in the original input, before header
    /// normalization. Validation errors may have no offset.
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
        for line in input.split_inclusive('\n') {
            if line.starts_with("diff --cc ")
                || line.starts_with("diff --combined ")
                || line.starts_with("@@@")
            {
                return Err(DiffError {
                    offset: Some(offset),
                    message: "combined merge diffs are unsupported".into(),
                });
            }
            if line.starts_with("diff --git ") && offset > chunk_start {
                chunks.push((chunk_start, &input[chunk_start..offset]));
                chunk_start = offset;
            }
            offset += line.len();
        }
        chunks.push((chunk_start, &input[chunk_start..]));
        for (base, chunk) in chunks {
            let mut normalized = String::new();
            let mut metadata = Vec::new();
            let mut binary = false;
            let mut in_hunk = false;
            let mut byte_map = Vec::new();
            let mut source_offset = base;
            for line in chunk.split_inclusive('\n') {
                if line.starts_with("@@ ") {
                    in_hunk = true;
                }
                if line.starts_with("diff --git ") {
                    in_hunk = false;
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
            let options = if normalized.starts_with("diff --git ") {
                ParseOptions::gitdiff()
            } else {
                ParseOptions::unidiff()
            };
            for parsed in PatchSet::parse(&normalized, options) {
                let patch = parsed.map_err(|e| {
                    let message = e.to_string();
                    let n = message
                        .split("at byte ")
                        .nth(1)
                        .and_then(|s| s.split(':').next())
                        .and_then(|s| s.parse::<usize>().ok());
                    DiffError {
                        offset: n
                            .map(|n| {
                                let line = byte_map
                                    .partition_point(|&(normalized, _)| normalized <= n)
                                    .saturating_sub(1);
                                let (normalized, original) = byte_map[line];
                                original + n.saturating_sub(normalized)
                            })
                            .or(Some(base)),
                        message,
                    }
                })?;
                let (old_path, new_path) = match patch.operation() {
                    FileOperation::Delete(p) => (Some(p.to_string()), None),
                    FileOperation::Create(p) => (None, Some(p.to_string())),
                    FileOperation::Modify { original, modified } => {
                        (Some(original.to_string()), Some(modified.to_string()))
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
                                diffy::Line::Context(s) => {
                                    (LineKind::Context, *s, Some(old), Some(new))
                                }
                                diffy::Line::Delete(s) => (LineKind::Delete, *s, Some(old), None),
                                diffy::Line::Insert(s) => (LineKind::Insert, *s, None, Some(new)),
                            };
                            lines.push(DiffLine::new(kind, a, b, content));
                            old += usize::from(a.is_some());
                            new += usize::from(b.is_some());
                        }
                        hunks.push(Hunk {
                            old: h.old_range().start()..old,
                            new: h.new_range().start()..new,
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
        }
        Self::new(files)
    }
}
