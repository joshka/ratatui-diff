//! Retained context navigation, controls, and reveal contracts.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, HitTest, Side, SourceBoundary, SourcePosition, SourceSelection,
    ViewMode,
};

fn fixture_document() -> DiffDocument {
    let old: String = (1..=40).map(|n| format!("line {n:02} 界\n")).collect();
    let new = old
        .replace("line 06", "changed 06")
        .replace("line 35", "changed 35");
    DiffDocument::compare(&old, &new, usize::MAX)
}

fn paint(
    document: &DiffDocument,
    state: &mut DiffState,
    radius: Option<usize>,
    mode: ViewMode,
    width: u16,
) -> Buffer {
    let area = Rect::new(2, 3, width, 20);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(document)
        .context_lines(radius)
        .mode(mode)
        .wrap(true))
        .render(area, &mut buffer, state);
    buffer
}

#[test]
fn exact_retained_counts_and_source_numbers_survive_modes() {
    let document = fixture_document();
    let mut state = DiffState::new();
    paint(&document, &mut state, None, ViewMode::Unified, 80);
    assert!(state.context_folds().is_empty());
    for mode in [ViewMode::Unified, ViewMode::Split] {
        paint(&document, &mut state, Some(3), mode, 80);
        assert_eq!(
            state
                .context_folds()
                .iter()
                .map(|fold| fold.line_count())
                .collect::<Vec<_>>(),
            [2, 22, 2]
        );
        assert_eq!(state.context_folds()[1].old, 10..32);
        assert_eq!(state.context_folds()[1].new, 10..32);
        let fold = state.context_folds()[1].clone();
        assert!(state.set_context_expanded(&fold, true));
        assert!(state.hit_test(2, 3).is_none());
        paint(&document, &mut state, Some(3), mode, 80);
        assert!(state.context_expanded(&fold));
        let position = SourcePosition {
            file: 0,
            side: Side::New,
            line: 20,
        };
        assert!(state.scroll_to_source(position));
        paint(&document, &mut state, Some(3), mode, 80);
        assert_eq!(state.source_at(state.offset(), Side::New), Some(position));
        state.set_context_expanded(&fold, false);
    }
}

#[test]
fn reveal_search_and_selection_expands_but_manual_collapse_stays() {
    let document = fixture_document();
    let mut state = DiffState::new();
    paint(&document, &mut state, Some(3), ViewMode::Split, 45);
    let fold = state.context_folds()[1].clone();
    state.set_search(&document, "line 20", Some(Side::New));
    state.next_match();
    paint(&document, &mut state, Some(3), ViewMode::Split, 45);
    assert!(state.context_expanded(&fold));
    state.set_context_expanded(&fold, false);
    paint(&document, &mut state, Some(3), ViewMode::Split, 45);
    assert!(!state.context_expanded(&fold));
    let focus = SourceBoundary {
        position: SourcePosition {
            file: 0,
            side: Side::New,
            line: 25,
        },
        byte: 0,
    };
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: focus,
            focus
        }
    ));
    paint(&document, &mut state, Some(3), ViewMode::Split, 45);
    assert!(!state.context_expanded(&fold));
    state.reveal_selection();
    paint(&document, &mut state, Some(3), ViewMode::Split, 45);
    assert!(state.context_expanded(&fold));
}

#[test]
fn zero_radius_stale_handles_and_fold_hits() {
    let document = fixture_document();
    let mut state = DiffState::new();
    paint(&document, &mut state, Some(0), ViewMode::Unified, 12);
    let fold = state.context_folds()[0].clone();
    assert_eq!(fold.line_count(), 5);
    let y = (3..23)
        .find(|&y| matches!(state.hit_test(2, y), Some(HitTest::Fold { .. })))
        .unwrap();
    assert_eq!(
        state.hit_test(2, y),
        Some(HitTest::Fold { fold: fold.clone() })
    );
    assert!(state.source_at(usize::from(y - 3), Side::New).is_none());
    let position = SourcePosition {
        file: 0,
        side: Side::Old,
        line: 3,
    };
    assert!(state.scroll_to_source(position));
    paint(&document, &mut state, Some(0), ViewMode::Unified, 12);
    assert!(state.context_expanded(&fold));
    paint(&document, &mut state, Some(3), ViewMode::Unified, 12);
    assert!(!state.set_context_expanded(&fold, true));
    let fold = state.context_folds()[0].clone();
    paint(
        &fixture_document(),
        &mut state,
        Some(3),
        ViewMode::Unified,
        12,
    );
    assert!(!state.set_context_expanded(&fold, true));
}

#[test]
fn unavailable_context_never_exposes_a_fold_or_source() {
    let old: String = (1..=40).map(|n| format!("line {n:02}\n")).collect();
    let new = old
        .replace("line 06", "change 06")
        .replace("line 35", "change 35");
    let document = DiffDocument::compare(&old, &new, 3);
    let mut state = DiffState::new();
    let buffer = paint(&document, &mut state, Some(3), ViewMode::Unified, 80);
    assert!(state.context_folds().is_empty());
    let y = (3..23)
        .find(|&y| {
            (2..82)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .contains("context unavailable")
        })
        .unwrap();
    assert_eq!(state.hit_test(2, y), Some(HitTest::Header { file: 0 }));
    assert!(state.source_at(usize::from(y - 3), Side::New).is_none());
    assert!(!state.scroll_to_source(SourcePosition {
        file: 0,
        side: Side::New,
        line: 20
    }));
}

#[test]
fn explicit_selection_reveal_takes_precedence_over_search_fold() {
    let document = fixture_document();
    let mut state = DiffState::new();
    paint(&document, &mut state, Some(3), ViewMode::Unified, 80);
    let leading = state.context_folds()[0].clone();
    let middle = state.context_folds()[1].clone();
    state.set_search(&document, "line 01", Some(Side::New));
    state.next_match();
    let focus = SourceBoundary {
        position: SourcePosition {
            file: 0,
            side: Side::New,
            line: 25,
        },
        byte: 0,
    };
    state.set_selection(
        &document,
        SourceSelection {
            anchor: focus,
            focus,
        },
    );
    state.reveal_selection();
    paint(&document, &mut state, Some(3), ViewMode::Unified, 80);
    assert!(state.context_expanded(&middle));
    assert!(!state.context_expanded(&leading));
}

#[test]
fn expanding_a_visible_summary_reveals_its_first_line() {
    let document = fixture_document();
    let mut state = DiffState::new();
    paint(&document, &mut state, Some(3), ViewMode::Unified, 80);
    let fold = state.context_folds()[0].clone();
    state.set_context_expanded(&fold, true);
    paint(&document, &mut state, Some(3), ViewMode::Unified, 80);
    assert_eq!(state.source_at(state.offset(), Side::Old).unwrap().line, 1);
}
