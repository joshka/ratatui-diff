//! Literal source search, prepared by the host outside frame rendering.

use crate::{DiffDocument, Side, SourcePosition, SourceRange};

#[derive(Debug, Default)]
pub(crate) struct Search {
    pub document: Option<u64>,
    pub query: String,
    pub side: Option<Side>,
    pub matches: Vec<SourceRange>,
    pub active: Option<usize>,
    pub reveal: bool,
    // Sorted source keys retain result indexes, independent of display mode.
    pub sources: Vec<(usize, usize, usize, usize)>,
}

impl Search {
    pub fn update(&mut self, document: &DiffDocument, query: &str, side: Option<Side>) {
        if self.document == Some(document.id) && self.query == query && self.side == side {
            return;
        }
        *self = Self {
            document: Some(document.id),
            query: query.to_owned(),
            side,
            ..Self::default()
        };
        if query.is_empty() {
            return;
        }
        for (file, data) in document.files().iter().enumerate() {
            for hunk in &data.hunks {
                for line in &hunk.lines {
                    let chosen = side.unwrap_or(if line.new.is_some() {
                        Side::New
                    } else {
                        Side::Old
                    });
                    let number = match chosen {
                        Side::Old => line.old,
                        Side::New => line.new,
                    };
                    let Some(number) = number else { continue };
                    for (start, text) in line.text.match_indices(query) {
                        let index = self.matches.len();
                        self.matches.push(SourceRange {
                            position: SourcePosition {
                                file,
                                side: chosen,
                                line: number,
                            },
                            bytes: start..start + text.len(),
                        });
                        // Shared context paints both source copies but counts only once.
                        for (source_side, source_line) in
                            [(Side::Old, line.old), (Side::New, line.new)]
                        {
                            if let Some(source_line) = source_line
                                && (side.is_none() || side == Some(source_side))
                            {
                                self.sources.push((
                                    file,
                                    side_key(source_side),
                                    source_line,
                                    index,
                                ));
                            }
                        }
                    }
                }
            }
        }
        self.sources.sort_unstable();
    }

    pub fn indexes(&self, position: SourcePosition) -> &[(usize, usize, usize, usize)] {
        let key = (position.file, side_key(position.side), position.line);
        let start = self
            .sources
            .partition_point(|&(f, s, l, _)| (f, s, l) < key);
        let end = self
            .sources
            .partition_point(|&(f, s, l, _)| (f, s, l) <= key);
        &self.sources[start..end]
    }
}

fn side_key(side: Side) -> usize {
    match side {
        Side::Old => 0,
        Side::New => 1,
    }
}
