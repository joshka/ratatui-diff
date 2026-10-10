//! Retained context and unavailable source remain distinct without another source store.

use ratatui_diff::{
    DiffDocument, DiffState, Side, SourceBoundary, SourcePosition, SourceSelection,
};

fn source() -> (String, String) {
    let old = (1..=25)
        .map(|line| format!("line {line} 界\r\n"))
        .collect::<String>();
    let new = old
        .replace("line 5 界", "changed five")
        .replace("line 21 界", "changed twenty-one");
    (old, new)
}

fn point(side: Side, line: usize, byte: usize) -> SourceBoundary {
    SourceBoundary {
        position: SourcePosition {
            file: 0,
            side,
            line,
        },
        byte,
    }
}

#[test]
fn full_context_preserves_source_numbers_endings_search_and_extraction() {
    let (old, new) = source();
    let document = DiffDocument::compare(&old, &new, usize::MAX);
    let file = &document.files()[0];
    assert_eq!(file.hunks.len(), 1);
    assert_eq!(file.hunks[0].old, 1..26);
    assert_eq!(file.hunks[0].new, 1..26);
    for (side, text) in [(Side::Old, &old), (Side::New, &new)] {
        let selection = SourceSelection {
            anchor: point(side, 1, 0),
            focus: point(side, 25, "line 25 界\r\n".len()),
        };
        assert_eq!(selection.text(&document).as_ref(), Some(text));
        let mut state = DiffState::new();
        state.set_search(&document, "line 13 界", Some(side));
        assert_eq!(state.search_matches().len(), 1);
        assert_eq!(
            state.search_matches()[0].position,
            point(side, 13, 0).position
        );
        assert_eq!(state.search_matches()[0].bytes, 0.."line 13 界".len());
    }
}

#[test]
fn default_context_does_not_reconstruct_discarded_lines() {
    let (old, new) = source();
    let document = DiffDocument::from_text(&old, &new);
    let explicit = DiffDocument::compare(&old, &new, 3);
    assert_eq!(document.files(), explicit.files());
    assert_eq!(document.files()[0].hunks.len(), 2);
    let mut state = DiffState::new();
    state.set_search(&document, "line 13 界", None);
    assert!(state.search_matches().is_empty());
    let selection = SourceSelection {
        anchor: point(Side::New, 8, 0),
        focus: point(Side::New, 18, 0),
    };
    assert_eq!(selection.text(&document), None);
}

#[test]
fn parsed_patch_gaps_are_unavailable_even_between_valid_selection_boundaries() {
    let patch = "--- a/file\n+++ b/file\n@@ -1,2 +1,2 @@\n retained before\n-old\n+new\n@@ -9,2 +9,2 @@\n retained after\n-old end\n+new end\n";
    let document = DiffDocument::parse(patch).unwrap();
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 9, "retained after".len()),
    };
    assert_eq!(selection.text(&document), None);
    let mut state = DiffState::new();
    state.set_search(&document, "retained", None);
    let numbers: Vec<_> = state
        .search_matches()
        .iter()
        .map(|found| found.position.line)
        .collect();
    assert_eq!(numbers, [1, 9]);
}

#[test]
fn full_context_keeps_identical_inputs_without_hunks_and_final_newline_bytes() {
    assert!(
        DiffDocument::compare("same\n", "same\n", usize::MAX).files()[0]
            .hunks
            .is_empty()
    );
    let document = DiffDocument::compare("old\nlast", "new\nlast", usize::MAX);
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 2, 4),
    };
    assert_eq!(selection.text(&document).as_deref(), Some("new\nlast"));
}
