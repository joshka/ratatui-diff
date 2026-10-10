//! Pinned headers retain file identity without becoming source rows.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, HitTest, Side, SourcePosition, ViewMode,
};

fn document() -> DiffDocument {
    let old: String = (1..=40)
        .map(|n| format!("line {n:02}: retained text\n"))
        .collect();
    let new = old.replace("line 20: retained", "line 20: changed");
    let files = ["src/first.rs", "src/second.rs"]
        .into_iter()
        .map(|path| {
            let mut file = DiffDocument::compare(&old, &new, usize::MAX).files()[0].clone();
            file.old_path = Some(path.into());
            file.new_path = Some(path.into());
            file
        })
        .collect();
    DiffDocument::new(files).unwrap()
}

fn paint(widget: Diff<'_>, state: &mut DiffState, width: u16, height: u16) -> Buffer {
    let area = Rect::new(2, 3, width, height);
    let mut buffer = Buffer::empty(area);
    (&widget).render(area, &mut buffer, state);
    buffer
}

fn text(buffer: &Buffer, y: u16) -> String {
    (buffer.area.x..buffer.area.right())
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn opt_in_keeps_header_styles_and_source_hits_in_both_modes() {
    let document = document();
    for mode in [ViewMode::Unified, ViewMode::Split] {
        let plain = Diff::new(&document)
            .mode(mode)
            .theme(DiffTheme::aardvark_ink());
        let sticky = plain.sticky_file_headers(true);
        let mut state = DiffState::new();
        let initial = paint(sticky, &mut state, 70, 10);
        let rows = state.row_count();
        state.scroll_lines(8);
        assert!(state.hit_test(2, 3).is_none());
        let buffer = paint(sticky, &mut state, 70, 10);
        assert_eq!(text(&buffer, 3), text(&initial, 3));
        for x in 2..72 {
            assert_eq!(buffer[(x, 3)], initial[(x, 3)]);
        }
        assert_eq!(state.row_count(), rows);
        assert!(matches!(state.hit_test(2,3), Some(HitTest::FileHeader{fold}) if fold.file()==0));
        let source = state.source_at(state.offset(), Side::Old).unwrap();
        assert!(
            matches!(state.hit_test(15,4), Some(HitTest::Source{old:Some(range),..}) if range.position==source)
        );
        let ordinary = paint(plain, &mut state, 70, 10);
        assert!(!text(&ordinary, 3).contains("src/first.rs"));
        assert!(matches!(
            state.hit_test(15, 3),
            Some(HitTest::Source { .. })
        ));
    }
}

#[test]
fn boundaries_fold_controls_and_source_reveal_use_the_current_file() {
    let document = document();
    let widget = Diff::new(&document)
        .sticky_file_headers(true)
        .context_lines(Some(3));
    let mut state = DiffState::new();
    paint(widget, &mut state, 60, 8);
    state.next_file();
    let buffer = paint(widget, &mut state, 60, 8);
    assert!(text(&buffer, 3).contains("src/second.rs"));
    assert!(!text(&buffer, 4).contains("src/second.rs"));
    let source = SourcePosition {
        file: 1,
        side: Side::New,
        line: 20,
    };
    assert!(state.scroll_to_source(source));
    paint(widget, &mut state, 60, 8);
    let Some(HitTest::FileHeader { fold }) = state.hit_test(2, 3) else {
        panic!("header control")
    };
    assert_eq!(fold.file(), 1);
    state.set_file_expanded(&fold, false);
    let buffer = paint(widget, &mut state, 60, 8);
    assert!((3..11).any(|y| text(&buffer, y).contains("▸ src/second.rs")));
    assert!(state.scroll_to_source(source));
    paint(widget, &mut state, 60, 8);
    assert!(state.file_expanded(&fold));
    assert!(text(&paint(widget, &mut state, 60, 8), 3).contains("src/second.rs"));
    assert!(state.scroll_to_source(SourcePosition { line: 2, ..source }));
    paint(widget, &mut state, 60, 8);
    assert!(
        state
            .context_folds()
            .iter()
            .any(|fold| state.context_expanded(fold))
    );
    state.previous_file();
    paint(widget, &mut state, 60, 8);
    assert!(matches!(state.hit_test(2,3),Some(HitTest::FileHeader{fold}) if fold.file()==1));
    state.previous_file();
    paint(widget, &mut state, 60, 8);
    assert!(matches!(state.hit_test(2,3),Some(HitTest::FileHeader{fold}) if fold.file()==0));
}

#[test]
fn final_source_remains_visible_through_wrap_resize_stats_and_replacement() {
    let document = document();
    let mut state = DiffState::new();
    for mode in [ViewMode::Unified, ViewMode::Split] {
        let widget = Diff::new(&document)
            .sticky_file_headers(true)
            .show_stats(true)
            .mode(mode)
            .wrap(true);
        for width in [70, 22, 9, 70] {
            paint(widget, &mut state, width, 10);
            state.end();
            let buffer = paint(widget, &mut state, width, 10);
            assert!(text(&buffer, 3).starts_with("2 files"));
            assert!(
                matches!(state.hit_test(2,4),Some(HitTest::FileHeader{fold}) if fold.file()==1)
            );
            assert!((5..13).any(|y| matches!(state.hit_test(2,y),Some(HitTest::Gutter{old:Some(position),..}) if position.file==1 && position.line==40)));
        }
        for height in [0, 1, 2, 3] {
            paint(widget, &mut state, 22, height);
        }
    }
    let replacement = DiffDocument::from_text("a\n", "b\n");
    let buffer = paint(
        Diff::new(&replacement).sticky_file_headers(true),
        &mut state,
        60,
        10,
    );
    assert_eq!(state.offset(), 0);
    assert!(!text(&buffer, 3).contains("src/second.rs"));
}
