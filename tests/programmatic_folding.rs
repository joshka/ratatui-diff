//! Programmatic review controls use the same folding state as host input events.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, Side, SourceBoundary, SourcePosition, SourceSelection,
};

fn review_document() -> DiffDocument {
    let old: String = (1..=20)
        .map(|line| format!("line {line:02} 界\r\n"))
        .collect();
    let new = old.replace("line 08", "changed 08");
    let compared = DiffDocument::compare(&old, &new, usize::MAX);
    let mut first = compared.files()[0].clone();
    first.old_path = Some("first.rs".into());
    first.new_path = first.old_path.clone();
    let mut second = first.clone();
    second.old_path = Some("second.rs".into());
    second.new_path = second.old_path.clone();
    DiffDocument::new(vec![first, second]).unwrap()
}

fn draw(document: &DiffDocument, state: &mut DiffState) {
    let area = Rect::new(0, 0, 60, 12);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(document).context_lines(Some(3));
    (&widget).render(area, &mut buffer, state);
}

#[test]
fn host_collapses_files_and_controls_nested_context_without_changing_selected_source() {
    let document = review_document();
    let mut state = DiffState::new();
    draw(&document, &mut state);
    let files = state.file_folds().to_vec();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|file| state.file_expanded(file)));
    let context = state.context_folds().to_vec();
    let nested = context
        .iter()
        .find(|fold| fold.file == 0 && fold.new.contains(&13))
        .unwrap();
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 12,
    };
    let selected = SourceSelection {
        anchor: SourceBoundary { position, byte: 0 },
        focus: SourceBoundary {
            position: SourcePosition {
                line: 14,
                ..position
            },
            byte: 0,
        },
    };
    assert!(state.set_selection(&document, selected));
    for file in &files {
        assert!(state.set_file_expanded(file, false));
    }
    draw(&document, &mut state);
    assert_eq!(state.row_count(), 2);
    assert_eq!(state.context_folds(), context);
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("line 12 界\r\nline 13 界\r\n")
    );

    // A host can prepare nested context without opening its containing file.
    assert!(state.set_context_expanded(nested, true));
    draw(&document, &mut state);
    assert!(!state.file_expanded(&files[0]));
    assert!(state.context_expanded(nested));
    assert_eq!(state.row_count(), 2);

    assert!(state.reveal_selection());
    draw(&document, &mut state);
    assert!(state.file_expanded(&files[0]));
    assert!(!state.file_expanded(&files[1]));
    assert!(state.context_expanded(nested));
    assert_eq!(state.selection(), Some(selected));
    assert_eq!(state.selected_text(&document), selected.text(&document));
}

#[test]
fn saved_file_state_is_applied_before_first_frame_and_rebound_on_replacement() {
    let document = review_document();
    let saved = document.file_fold(1).unwrap();
    assert_eq!(saved.file(), 1);
    assert!(document.file_fold(2).is_none());
    let mut state = DiffState::new();
    assert!(state.set_file_expanded(&saved, false));
    assert!(!state.file_expanded(&saved));
    draw(&document, &mut state);
    assert!(!state.file_expanded(&saved));
    assert!(state.file_expanded(&document.file_fold(0).unwrap()));
    assert!((0..state.row_count()).all(|row| {
        state
            .source_at(row, Side::New)
            .is_none_or(|position| position.file != 1)
    }));

    let area = Rect::new(0, 0, 24, 8);
    let mut buffer = Buffer::empty(area);
    let clone = document.clone();
    let widget = Diff::new(&clone).wrap(true);
    (&widget).render(area, &mut buffer, &mut state);
    assert!(!state.file_expanded(&saved));

    let replacement = review_document();
    let fresh = replacement.file_fold(1).unwrap();
    assert!(!state.set_file_expanded(&fresh, false));
    draw(&replacement, &mut state);
    assert!(state.file_expanded(&fresh));
    assert!(!state.file_expanded(&saved));
    assert!(!state.set_file_expanded(&saved, false));
}
