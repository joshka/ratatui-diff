//! Public source/display coordinate contracts.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, HitTest, Side, SourcePosition, ViewMode};

fn render(widget: &Diff<'_>, area: Rect, state: &mut DiffState) {
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer, state);
}

fn bytes(state: &DiffState, x: u16, y: u16) -> std::ops::Range<usize> {
    let Some(HitTest::Source { old, new }) = state.hit_test(x, y) else {
        panic!("expected source at {x},{y}: {:?}", state.hit_test(x, y));
    };
    new.or(old).unwrap().bytes
}

#[test]
fn unicode_tabs_controls_and_notation_keep_source_bytes() {
    let document = DiffDocument::from_text("", "\t界e\u{301}\u{1}\u{302}");
    let widget = Diff::new(&document).line_numbers(false).whitespace(true);
    let mut state = DiffState::new();
    let area = Rect::new(7, 9, 40, 8);
    assert_eq!(state.hit_test(8, 11), None);
    render(&widget, area, &mut state);
    assert_eq!(state.hit_test(7, 9), Some(HitTest::Header { file: 0 }));
    for x in 8..12 {
        assert_eq!(bytes(&state, x, 11), 0..1);
    }
    for x in 12..14 {
        assert_eq!(bytes(&state, x, 11), 1..4);
    }
    assert_eq!(bytes(&state, 14, 11), 4..7);
    for x in 15..20 {
        assert_eq!(bytes(&state, x, 11), 7..8);
    }
    assert_eq!(bytes(&state, 20, 11), 8..10);
    assert!(matches!(
        state.hit_test(21, 11),
        Some(HitTest::FinalNewline { .. })
    ));
    assert_eq!(state.hit_test(23, 11), Some(HitTest::Padding));
    assert_eq!(state.hit_test(6, 11), None);
    assert_eq!(state.hit_test(47, 11), None);
}

#[test]
fn context_hits_include_both_sides_and_gutters_have_no_bytes() {
    let document = DiffDocument::parse("--- a/x\n+++ b/x\n@@ -1 +1 @@\n abc\n").unwrap();
    let mut state = DiffState::new();
    render(&Diff::new(&document), Rect::new(0, 0, 20, 6), &mut state);
    assert!(matches!(
        state.hit_test(0, 2),
        Some(HitTest::Gutter {
            old: Some(_),
            new: Some(_)
        })
    ));
    let Some(HitTest::Source {
        old: Some(old),
        new: Some(new),
    }) = state.hit_test(5, 2)
    else {
        panic!()
    };
    assert_eq!(old.bytes, 0..1);
    assert_eq!(old.position.side, Side::Old);
    assert_eq!(new.position.side, Side::New);
}

#[test]
fn split_padding_clipping_and_wrapping_never_claim_source() {
    let document = DiffDocument::from_text("界abc\n", "x\n");
    let mut state = DiffState::new();
    let widget = Diff::new(&document)
        .mode(ViewMode::Split)
        .line_numbers(false)
        .wrap(true);
    render(&widget, Rect::new(0, 0, 8, 100), &mut state);
    let y = (0..state.row_count())
        .find(|&n| state.source_at(n, Side::Old).is_some())
        .unwrap() as u16;
    assert_eq!(bytes(&state, 1, y), 0..3);
    assert_eq!(bytes(&state, 2, y), 0..3);
    assert_eq!(state.hit_test(3, y), Some(HitTest::Separator));
    assert_eq!(bytes(&state, 5, y), 0..1);
    assert_eq!(state.hit_test(7, y), Some(HitTest::Padding));
    assert_eq!(state.hit_test(5, y + 1), Some(HitTest::Padding));
    render(&widget, Rect::new(0, 0, 5, 100), &mut state);
    let y = (0..state.row_count())
        .find(|&n| state.source_at(n, Side::Old).is_some())
        .unwrap() as u16;
    assert_eq!(state.hit_test(1, y), Some(HitTest::Padding));
    let widget = widget.wrap(false);
    render(&widget, Rect::new(0, 0, 8, 100), &mut state);
    let y = (0..state.row_count())
        .find(|&n| state.source_at(n, Side::Old).is_some())
        .unwrap() as u16;
    state.scroll_horizontal(1);
    assert_eq!(state.hit_test(1, y), None);
    render(&widget, Rect::new(0, 0, 8, 100), &mut state);
    assert_eq!(state.hit_test(1, y), Some(HitTest::Padding));
    assert_eq!(bytes(&state, 2, y), 3..4);
}

#[test]
fn redraw_refreshes_origin_resize_and_document_identity() {
    let document = DiffDocument::from_text("", "abcdef\nghijkl\n");
    let widget = Diff::new(&document).line_numbers(false).wrap(true);
    let mut state = DiffState::new();
    render(&widget, Rect::new(0, 0, 5, 3), &mut state);
    state.scroll_lines(1);
    assert_eq!(state.hit_test(1, 2), None);
    render(&widget, Rect::new(10, 10, 10, 5), &mut state);
    assert_eq!(state.hit_test(1, 2), None);
    state.invalidate_hit_testing();
    assert_eq!(state.hit_test(11, 12), None);
    let replacement = DiffDocument::from_text("", "z\n");
    render(
        &Diff::new(&replacement).line_numbers(false),
        Rect::new(10, 10, 10, 5),
        &mut state,
    );
    assert_eq!(state.offset(), 0);
    assert_eq!(bytes(&state, 11, 12), 0..1);
    assert!(!state.scroll_to_source(SourcePosition {
        file: 0,
        side: Side::New,
        line: 99
    }));
}

#[test]
fn omitted_patch_context_and_zero_width_have_no_source_hit() {
    let document = DiffDocument::parse("--- a/x\n+++ b/x\n@@ -10 +10 @@\n-a\n+b\n").unwrap();
    let mut state = DiffState::new();
    render(&Diff::new(&document), Rect::new(3, 4, 0, 5), &mut state);
    assert_eq!(state.hit_test(3, 6), None);
    assert!(!state.scroll_to_source(SourcePosition {
        file: 0,
        side: Side::Old,
        line: 1
    }));
}

#[test]
fn empty_lines_and_missing_split_partners_distinguish_identity_from_padding() {
    let document = DiffDocument::from_text("", "\nmore\n");
    let widget = Diff::new(&document)
        .mode(ViewMode::Split)
        .line_numbers(false);
    let mut state = DiffState::new();
    render(&widget, Rect::new(0, 0, 31, 8), &mut state);
    assert_eq!(state.hit_test(1, 2), Some(HitTest::Padding));
    let Some(HitTest::Gutter {
        old: None,
        new: Some(position),
    }) = state.hit_test(16, 2)
    else {
        panic!("empty source line keeps its gutter identity");
    };
    assert_eq!(position.line, 1);
    assert_eq!(state.hit_test(17, 2), Some(HitTest::Padding));
    let Some(HitTest::Source {
        old: None,
        new: Some(source),
    }) = state.hit_test(17, 3)
    else {
        panic!("insertion has only a new-side range");
    };
    assert_eq!(source.position.line, 2);
    assert_eq!(
        &document.files()[0].hunks[0].lines[1].text[source.bytes],
        "m"
    );
}
