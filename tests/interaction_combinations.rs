//! Combined host workflows preserve source identity independently of display geometry.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, HitTest, SelectionMotion, Side, SourceBoundary,
    SourcePosition, SourceSelection, ViewMode,
};

fn boundary(side: Side, line: usize, byte: usize) -> SourceBoundary {
    SourceBoundary {
        position: SourcePosition {
            file: 0,
            side,
            line,
        },
        byte,
    }
}

fn draw(
    document: &DiffDocument,
    state: &mut DiffState,
    area: Rect,
    mode: ViewMode,
    wrap: bool,
) -> Buffer {
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(document)
        .mode(mode)
        .wrap(wrap)
        .whitespace(true)
        .theme(DiffTheme::aardvark_ink())
        .selection_style(
            Style::default()
                .bg(Color::Blue)
                .remove_modifier(Modifier::REVERSED),
        );
    (&widget).render(area, &mut buffer, state);
    buffer
}

#[test]
fn searched_unicode_selection_survives_horizontal_reveal_and_wrapped_resize() {
    let prefix = "x".repeat(100);
    let selected = "\t界e\u{301}\u{1}";
    let text = format!("{prefix}{selected}\nnext\n");
    let document = DiffDocument::from_text("", &text);
    let mut state = DiffState::new();
    state.set_search(&document, "界e\u{301}", Some(Side::New));
    let selection = SourceSelection {
        anchor: boundary(Side::New, 1, prefix.len()),
        focus: boundary(Side::New, 2, 4),
    };
    assert!(state.set_selection(&document, selection));
    draw(
        &document,
        &mut state,
        Rect::new(3, 2, 40, 12),
        ViewMode::Unified,
        false,
    );
    assert!(state.next_match());

    for (mode, wrap, width) in [
        (ViewMode::Unified, false, 40),
        (ViewMode::Split, true, 31),
        (ViewMode::Unified, true, 19),
    ] {
        let area = Rect::new(3, 2, width, 24);
        state.invalidate_hit_testing();
        assert_eq!(state.hit_test(area.x, area.y), None);
        let buffer = draw(&document, &mut state, area, mode, wrap);
        assert_eq!(state.selection(), Some(selection));
        assert_eq!(
            state.selected_text(&document).as_deref(),
            Some("\t界e\u{301}\u{1}\nnext")
        );
        assert_eq!(state.active_match(), Some(0));
        assert_eq!(state.search_matches()[0].bytes, 101..107);
        assert_eq!(state.horizontal_offset() > 0, !wrap);
        let mut matched_cells = 0;
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(HitTest::Source {
                    new: Some(range), ..
                }) = state.hit_test(x, y)
                {
                    let selected = range.position.line == 2 || range.bytes.start >= 100;
                    assert_eq!(buffer[(x, y)].bg == Color::Blue, selected);
                    if range.position.line == 1
                        && range.bytes.start >= 101
                        && range.bytes.end <= 107
                    {
                        let cell = &buffer[(x, y)];
                        assert_eq!(cell.bg, Color::Blue);
                        assert!(!cell.modifier.contains(Modifier::REVERSED));
                        assert!(range.bytes == (101..104) || range.bytes == (104..107));
                        matched_cells += 1;
                    }
                }
            }
        }
        assert!(
            matched_cells > 0,
            "active search must remain visible after {mode:?} resize"
        );
    }
}

#[test]
fn context_hits_keep_mode_specific_identity_with_search_and_selection() {
    let document = DiffDocument::from_text("\t界e\u{301}\nold\n", "\t界e\u{301}\nnew\n");
    let mut state = DiffState::new();
    state.set_search(&document, "\u{301}", Some(Side::Old));
    assert!(state.next_match());
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: boundary(Side::New, 1, 1),
            focus: boundary(Side::New, 1, 7),
        }
    ));
    for mode in [ViewMode::Unified, ViewMode::Split] {
        let area = Rect::new(5, 3, 31, 12);
        let buffer = draw(&document, &mut state, area, mode, true);
        let mut contexts = 0;
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(HitTest::Source { old, new }) = state.hit_test(x, y) {
                    let range = old.as_ref().or(new.as_ref()).unwrap();
                    if range.position.line != 1 || range.bytes != (4..7) {
                        continue;
                    }
                    assert_eq!(old.is_some() && new.is_some(), mode == ViewMode::Unified);
                    assert_eq!(buffer[(x, y)].bg == Color::Blue, new.is_some());
                    assert_eq!(
                        buffer[(x, y)].modifier.contains(Modifier::REVERSED),
                        new.is_none()
                    );
                    contexts += 1;
                }
            }
        }
        assert_eq!(contexts, if mode == ViewMode::Unified { 1 } else { 2 });
        assert_eq!(
            state.selected_text(&document).as_deref(),
            Some("界e\u{301}")
        );
    }
}

#[test]
fn missing_context_and_replacement_do_not_reuse_interaction_coordinates() {
    let document = DiffDocument::parse(
        "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+needle\n@@ -3 +3 @@\n-c\n+needle\n",
    )
    .unwrap();
    let mut state = DiffState::new();
    state.set_search(&document, "needle", Some(Side::New));
    assert!(state.next_match());
    let caret = boundary(Side::New, 1, 6);
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret
        }
    ));
    assert!(state.extend_selection(&document, SelectionMotion::Next));
    assert!(!state.extend_selection(&document, SelectionMotion::Next));
    assert!(!state.set_selection(
        &document,
        SourceSelection {
            anchor: boundary(Side::New, 1, 0),
            focus: boundary(Side::New, 3, 6),
        }
    ));
    let area = Rect::new(4, 2, 40, 10);
    draw(&document, &mut state, area, ViewMode::Unified, false);
    assert!(state.next_match());
    draw(&document, &mut state, area, ViewMode::Split, true);
    assert!(
        state.previous_match(),
        "leave a pending reveal on the replaced document"
    );
    let replacement = DiffDocument::from_text("", "fresh\n");
    state.invalidate_hit_testing();
    assert_eq!(state.selected_text(&replacement), None);
    draw(&replacement, &mut state, area, ViewMode::Unified, false);
    assert!(state.search_matches().is_empty());
    assert_eq!(state.active_match(), None);
    assert_eq!(state.selection(), None);
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(HitTest::Source {
                new: Some(range), ..
            }) = state.hit_test(x, y)
            {
                assert_eq!(range.position.line, 1);
                assert!(range.bytes.end <= 5);
            }
        }
    }
}

#[test]
fn successful_search_navigation_invalidates_the_last_frame() {
    let document = DiffDocument::from_text("", "first needle\nsecond needle\n");
    let mut state = DiffState::new();
    state.set_search(&document, "needle", None);
    let area = Rect::new(4, 2, 40, 10);
    for forward in [true, false] {
        draw(&document, &mut state, area, ViewMode::Unified, false);
        assert!(state.hit_test(area.x, area.y).is_some());
        assert!(if forward {
            state.next_match()
        } else {
            state.previous_match()
        });
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                assert_eq!(
                    state.hit_test(x, y),
                    None,
                    "navigation must retire every cell in the previous frame"
                );
            }
        }
    }
}
