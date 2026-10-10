//! Whole-file header controls, source reveal, and navigation anchors.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, HitTest, Side, SourcePosition, ViewMode};

fn document() -> DiffDocument {
    let old: String = (1..=40)
        .map(|n| format!("rule {n:02}: retained\n"))
        .collect();
    let new = old
        .replace("rule 06: retained", "rule 06: changed")
        .replace("rule 35: retained", "rule 35: changed");
    let mut files = Vec::new();
    for path in ["src/first.rs", "src/second.rs"] {
        let mut file = DiffDocument::compare(&old, &new, usize::MAX).files()[0].clone();
        file.old_path = Some(path.into());
        file.new_path = Some(path.into());
        files.push(file);
    }
    DiffDocument::new(files).unwrap()
}

fn paint(document: &DiffDocument, state: &mut DiffState, mode: ViewMode, width: u16) -> Buffer {
    let area = Rect::new(2, 3, width, 10);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(document)
        .context_lines(Some(3))
        .mode(mode)
        .wrap(true))
        .render(area, &mut buffer, state);
    buffer
}

#[test]
fn closed_files_keep_headers_counts_and_no_source_hits() {
    let document = document();
    let mut state = DiffState::new();
    paint(&document, &mut state, ViewMode::Unified, 60);
    for fold in state.file_folds().to_vec() {
        assert!(state.set_file_expanded(&fold, false));
    }
    let buffer = paint(&document, &mut state, ViewMode::Split, 60);
    assert_eq!(state.row_count(), 2);
    let header: String = (2..62).map(|x| buffer[(x, 3)].symbol()).collect();
    assert!(header.contains("▸ src/first.rs · +2 −2"));
    assert!(matches!(state.hit_test(2,3),Some(HitTest::FileHeader{fold}) if fold.file()==0));
    assert!(state.source_at(0, Side::New).is_none());
    assert!(matches!(state.hit_test(2, 5), Some(HitTest::Padding)));
}

#[test]
fn collapse_current_file_keeps_header_and_previous_file_preserves_source() {
    let document = document();
    let mut state = DiffState::new();
    paint(&document, &mut state, ViewMode::Unified, 60);
    let first = state.file_folds()[0];
    let second = state.file_folds()[1];
    let position = SourcePosition {
        file: 1,
        side: Side::New,
        line: 6,
    };
    assert!(state.scroll_to_source(position));
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(state.set_file_expanded(&first, false));
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert_eq!(state.source_at(state.offset(), Side::New), Some(position));
    assert!(state.set_file_expanded(&second, false));
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(matches!(state.hit_test(2,4),Some(HitTest::FileHeader{fold}) if fold==second));
}

#[test]
fn source_and_hunk_navigation_open_file_without_losing_context_preferences() {
    let document = document();
    let mut state = DiffState::new();
    paint(&document, &mut state, ViewMode::Unified, 60);
    let file = state.file_folds()[0];
    let context = state.context_folds()[0].clone();
    state.set_context_expanded(&context, true);
    paint(&document, &mut state, ViewMode::Unified, 60);
    state.set_file_expanded(&file, false);
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(state.context_expanded(&context));
    state.set_context_expanded(&context, false);
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(!state.file_expanded(&file));
    state.next_hunk();
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(state.file_expanded(&file));
    assert!(matches!(
        state.hit_test(2, 3),
        Some(HitTest::Header { file: 0 })
    ));
    state.set_file_expanded(&file, false);
    paint(&document, &mut state, ViewMode::Unified, 60);
    let position = SourcePosition {
        file: 0,
        side: Side::Old,
        line: 1,
    };
    assert!(state.scroll_to_source(position));
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert!(state.file_expanded(&file));
    assert!(state.context_expanded(&context));
    assert_eq!(state.source_at(state.offset(), Side::Old), Some(position));
}

#[test]
fn expanding_all_closed_files_preserves_the_first_header() {
    let document = document();
    let mut state = DiffState::new();
    paint(&document, &mut state, ViewMode::Unified, 60);
    for fold in state.file_folds().to_vec() {
        state.set_file_expanded(&fold, false);
    }
    paint(&document, &mut state, ViewMode::Unified, 60);
    let first = state.file_folds()[0];
    for fold in state.file_folds().to_vec() {
        state.set_file_expanded(&fold, true);
    }
    paint(&document, &mut state, ViewMode::Unified, 60);
    assert_eq!(state.offset(), 0);
    assert_eq!(
        state.hit_test(2, 3),
        Some(HitTest::FileHeader { fold: first })
    );
    assert!(
        state
            .file_folds()
            .iter()
            .all(|fold| state.file_expanded(fold))
    );
}
