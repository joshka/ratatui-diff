//! File presentation controls preserve source-based interaction state.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, HitTest, Side, SourceBoundary, SourcePosition,
    SourceSelection, ViewMode,
};

const AREA: Rect = Rect::new(2, 3, 80, 18);

#[test]
fn collapsed_file_headers_keep_identity_without_source_coordinates() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    assert_eq!(state.file_folds().len(), 5);
    assert!(
        state
            .file_folds()
            .iter()
            .all(|fold| state.file_expanded(fold))
    );
    let expanded_rows = state.row_count();
    let first = state.file_folds()[0];
    assert!(state.set_file_expanded(&first, false));
    assert!(state.hit_test(AREA.x, AREA.y).is_none());
    let buffer = paint(&widget, &mut state, AREA);
    assert!(state.row_count() < expanded_rows);
    assert!(!state.file_expanded(&first));
    assert!(state.set_file_expanded(&first, false));
    assert!(matches!(
        state.hit_test(AREA.x, AREA.y),
        Some(HitTest::FileHeader { fold }) if fold == first
    ));
    assert!(row_text(&buffer, AREA, AREA.y).contains("alpha.txt"));
    for side in [Side::Old, Side::New] {
        assert!(state.source_at(state.offset(), side).is_none());
    }
    assert!(
        !visible_sources(&state, AREA)
            .iter()
            .any(|source| source.file == 0)
    );
}

#[test]
fn closing_a_file_preserves_context_choices_through_presentation_changes() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    let first = state.file_folds()[0];
    let middle = state.context_folds()[1].clone();
    assert!(state.set_context_expanded(&middle, true));
    paint(&widget, &mut state, AREA);
    assert!(state.set_file_expanded(&first, false));
    for (mode, width, wrap) in [
        (ViewMode::Unified, 1, false),
        (ViewMode::Split, 11, true),
        (ViewMode::Unified, 51, true),
        (ViewMode::Split, 80, false),
    ] {
        let area = Rect::new(5, 2, width, 8);
        let widget = Diff::new(&document)
            .context_lines(Some(3))
            .mode(mode)
            .wrap(wrap)
            .whitespace(true)
            .theme(DiffTheme::monochrome());
        paint(&widget, &mut state, area);
        assert!(!state.file_expanded(&first), "{mode:?}, width {width}");
        assert!(state.context_expanded(&middle));
        assert!(state.context_folds().contains(&middle));
        assert!(state.source_at(state.offset(), Side::New).is_none());
    }
    state.set_file_expanded(&first, true);
    paint(
        &Diff::new(&document).context_lines(Some(3)),
        &mut state,
        AREA,
    );
    assert!(state.context_expanded(&middle));
    state.set_file_expanded(&first, false);
    paint(
        &Diff::new(&document).context_lines(Some(2)),
        &mut state,
        AREA,
    );
    assert!(!state.file_expanded(&first));
    assert!(
        !state.context_expanded(&middle),
        "a radius change invalidates context handles only"
    );
}

#[test]
fn hidden_source_selection_and_query_stay_closed_until_explicit_reveal() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3)).wrap(true);
    paint(&widget, &mut state, AREA);
    let first = state.file_folds()[0];
    let middle = state.context_folds()[1].clone();
    let selection = SourceSelection {
        anchor: boundary(0, 20, 0),
        focus: boundary(0, 21, "alpha 21\t界e\u{301}".len()),
    };
    state.set_file_expanded(&first, false);
    assert!(state.set_selection(&document, selection));
    state.set_search(&document, "alpha 20", Some(Side::New));
    paint(&widget, &mut state, AREA);
    assert!(!state.file_expanded(&first));
    assert!(!state.context_expanded(&middle));
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("alpha 20\t界e\u{301}\nalpha 21\t界e\u{301}")
    );
    assert!(state.reveal_selection());
    let area = Rect::new(2, 3, 31, 12);
    paint(&widget, &mut state, area);
    assert!(state.file_expanded(&first));
    assert!(state.context_expanded(&middle));
    assert_eq!(state.selection(), Some(selection));
    assert!(visible_sources(&state, area).contains(&selection.focus.position));
}

#[test]
fn source_and_search_navigation_open_both_folding_layers() {
    let document = fixture_document();
    for search in [false, true] {
        let mut state = DiffState::new();
        let widget = Diff::new(&document).context_lines(Some(3));
        paint(&widget, &mut state, AREA);
        let first = state.file_folds()[0];
        let middle = state.context_folds()[1].clone();
        state.set_file_expanded(&first, false);
        paint(&widget, &mut state, AREA);
        let target = boundary(0, 20, 0).position;
        if search {
            state.set_search(&document, "alpha 20", Some(Side::New));
            assert!(state.next_match());
        } else {
            assert!(state.scroll_to_source(target));
        }
        assert!(state.hit_test(AREA.x, AREA.y).is_none());
        paint(&widget, &mut state, AREA);
        assert!(state.file_expanded(&first));
        assert!(state.context_expanded(&middle));
        assert!(visible_sources(&state, AREA).contains(&target));
    }
}

#[test]
fn manual_file_collapse_cancels_a_pending_search_reveal() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    let first = state.file_folds()[0];
    state.set_search(&document, "alpha 20", Some(Side::New));
    assert!(state.next_match());
    assert!(state.set_file_expanded(&first, false));
    paint(&widget, &mut state, AREA);
    assert!(!state.file_expanded(&first));
    assert_eq!(state.active_match(), Some(0));
    assert_eq!(state.search_query(), "alpha 20");
    assert!(
        !visible_sources(&state, AREA)
            .iter()
            .any(|source| source.file == 0)
    );
    assert!(state.next_match());
    paint(&widget, &mut state, AREA);
    assert!(state.file_expanded(&first));
}

#[test]
fn selection_reveal_opens_its_file_before_a_pending_search_in_another_file() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    let folds = state.file_folds().to_vec();
    for fold in &folds {
        state.set_file_expanded(fold, false);
    }
    paint(&widget, &mut state, AREA);
    state.set_search(&document, "beta 20", Some(Side::New));
    assert!(state.next_match());
    let caret = boundary(0, 20, 0);
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret
        }
    ));
    assert!(state.reveal_selection());
    paint(&widget, &mut state, AREA);
    assert!(state.file_expanded(&folds[0]));
    assert!(!state.file_expanded(&folds[1]));
    assert!(visible_sources(&state, AREA).contains(&caret.position));
}

#[test]
fn manual_file_collapse_cancels_pending_source_and_selection_reveals() {
    let document = fixture_document();
    for selection in [false, true] {
        let mut state = DiffState::new();
        let widget = Diff::new(&document).context_lines(Some(3));
        paint(&widget, &mut state, AREA);
        let first = state.file_folds()[0];
        let caret = boundary(0, 20, 0);
        if selection {
            state.set_selection(
                &document,
                SourceSelection {
                    anchor: caret,
                    focus: caret,
                },
            );
            assert!(state.reveal_selection());
        } else {
            assert!(state.scroll_to_source(caret.position));
        }
        state.set_file_expanded(&first, false);
        paint(&widget, &mut state, AREA);
        assert!(!state.file_expanded(&first));
        assert!(
            !visible_sources(&state, AREA)
                .iter()
                .any(|source| source.file == 0)
        );
        if selection {
            assert!(
                state.selection().is_some(),
                "collapse hides the body, not the selection"
            );
        }
    }
}

#[test]
fn file_navigation_keeps_collapsed_files_closed_but_hunk_navigation_opens_them() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    let folds = state.file_folds().to_vec();
    for fold in &folds {
        state.set_file_expanded(fold, false);
    }
    let area = Rect::new(2, 3, 80, 2);
    paint(&widget, &mut state, area);
    state.next_file();
    paint(&widget, &mut state, area);
    assert!(matches!(
        state.hit_test(area.x, area.y),
        Some(HitTest::FileHeader { fold }) if fold == folds[1]
    ));
    assert!(folds.iter().all(|fold| !state.file_expanded(fold)));
    state.previous_file();
    paint(&widget, &mut state, area);
    assert!(matches!(
        state.hit_test(area.x, area.y),
        Some(HitTest::FileHeader { fold }) if fold == folds[0]
    ));
    state.next_hunk();
    let buffer = paint(&widget, &mut state, area);
    assert!(state.file_expanded(&folds[0]));
    assert!(matches!(
        state.hit_test(area.x, area.y),
        Some(HitTest::Header { file: 0 })
    ));
    assert!(row_text(&buffer, area, area.y).contains("@@"));
    assert!(folds[1..].iter().all(|fold| !state.file_expanded(fold)));
}

#[test]
fn folding_an_earlier_file_preserves_the_current_source_anchor() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    let area = Rect::new(2, 3, 80, 4);
    paint(&widget, &mut state, area);
    let first = state.file_folds()[0];
    let target = boundary(1, 20, 0).position;
    assert!(state.scroll_to_source(target));
    paint(&widget, &mut state, area);
    assert_eq!(state.source_at(state.offset(), Side::New), Some(target));
    state.set_file_expanded(&first, false);
    paint(&widget, &mut state, area);
    assert_eq!(state.source_at(state.offset(), Side::New), Some(target));
}

#[test]
fn folding_the_current_file_returns_to_its_header() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    let area = Rect::new(2, 3, 80, 4);
    paint(&widget, &mut state, area);
    let current = state.file_folds()[1];
    assert!(state.scroll_to_source(boundary(1, 20, 0).position));
    paint(&widget, &mut state, area);
    state.set_file_expanded(&current, false);
    paint(&widget, &mut state, area);
    assert!(matches!(
        state.hit_test(area.x, area.y),
        Some(HitTest::FileHeader { fold }) if fold == current
    ));
    assert!(state.source_at(state.offset(), Side::New).is_none());
}

#[test]
fn replacement_rejects_old_handles_and_resets_source_interaction() {
    let document = fixture_document();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(3));
    paint(&widget, &mut state, AREA);
    let old = state.file_folds()[0];
    let caret = boundary(0, 20, 0);
    state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret,
        },
    );
    state.set_search(&document, "alpha 20", Some(Side::New));
    state.next_match();
    state.set_file_expanded(&old, false);
    let replacement = fixture_document();
    paint(
        &Diff::new(&replacement).context_lines(Some(3)),
        &mut state,
        AREA,
    );
    assert!(!state.file_expanded(&old));
    assert!(!state.set_file_expanded(&old, false));
    assert!(!state.set_file_expanded(&old, true));
    assert!(
        state
            .file_folds()
            .iter()
            .all(|fold| state.file_expanded(fold))
    );
    assert_eq!(state.selection(), None);
    assert_eq!(state.active_match(), None);
    assert!(state.search_query().is_empty());
    assert_eq!(state.offset(), 0);
}

#[test]
fn unavailable_patch_context_is_never_recovered_by_expanding_a_file() {
    let document = DiffDocument::parse(
        "--- a/gap.txt\n+++ b/gap.txt\n@@ -1 +1 @@\n-old\n+new\n@@ -3 +3 @@\n-end\n+last\n",
    )
    .unwrap();
    let mut state = DiffState::new();
    let widget = Diff::new(&document).context_lines(Some(0));
    paint(&widget, &mut state, AREA);
    let first = state.file_folds()[0];
    let missing = boundary(0, 2, 0).position;
    let across_gap = SourceSelection {
        anchor: boundary(0, 1, 0),
        focus: boundary(0, 3, 4),
    };
    state.set_file_expanded(&first, false);
    paint(&widget, &mut state, AREA);
    assert!(!state.scroll_to_source(missing));
    assert!(!state.file_expanded(&first));
    assert!(!state.set_selection(&document, across_gap));
    state.set_file_expanded(&first, true);
    let buffer = paint(&widget, &mut state, AREA);
    assert!(state.context_folds().is_empty());
    assert!(!state.scroll_to_source(missing));
    assert!(!state.set_selection(&document, across_gap));
    assert!(buffer.content.iter().any(|cell| cell.symbol() == "…"));
    let y = (AREA.y..AREA.bottom())
        .find(|&y| row_text(&buffer, AREA, y).contains("context unavailable"))
        .unwrap();
    assert_eq!(state.hit_test(AREA.x, y), Some(HitTest::Header { file: 0 }));
}

fn fixture_document() -> DiffDocument {
    let mut files = Vec::new();
    for name in ["alpha", "beta", "gamma"] {
        let old: String = (1..=40)
            .map(|n| format!("{name} {n:02}\t界e\u{301}\n"))
            .collect();
        let new = old
            .replace(&format!("{name} 06"), &format!("changed {name} 06"))
            .replace(&format!("{name} 35"), &format!("changed {name} 35"));
        let mut file = DiffDocument::compare(&old, &new, usize::MAX).files()[0].clone();
        file.old_path = Some(format!("{name}.txt"));
        file.new_path = Some(format!("{name}.txt"));
        files.push(file);
    }
    let summaries = DiffDocument::parse(
        "diff --git a/banner.png b/banner.png\nBinary files a/banner.png and b/banner.png differ\n\
         diff --git a/check.sh b/check.sh\nold mode 100644\nnew mode 100755\n",
    )
    .unwrap();
    files.extend_from_slice(summaries.files());
    DiffDocument::new(files).unwrap()
}

fn paint(widget: &Diff<'_>, state: &mut DiffState, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer, state);
    buffer
}

fn boundary(file: usize, line: usize, byte: usize) -> SourceBoundary {
    SourceBoundary {
        position: SourcePosition {
            file,
            side: Side::New,
            line,
        },
        byte,
    }
}

fn visible_sources(state: &DiffState, area: Rect) -> Vec<SourcePosition> {
    (0..usize::from(area.height))
        .filter_map(|row| state.source_at(state.offset() + row, Side::New))
        .collect()
}

fn row_text(buffer: &Buffer, area: Rect, y: u16) -> String {
    (area.x..area.right())
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}
