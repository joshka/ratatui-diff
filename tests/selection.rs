//! Source fidelity and selection boundary contracts.
use ratatui_diff::{DiffDocument, Side, SourceBoundary, SourcePosition, SourceSelection};

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
fn extract_original_bytes_in_both_directions() {
    let document = DiffDocument::from_text("", "\t界e\u{301}\u{1b}\r\nlast");
    let selection = SourceSelection {
        anchor: point(Side::New, 2, 4),
        focus: point(Side::New, 1, 0),
    };
    assert_eq!(
        selection.text(&document).as_deref(),
        Some("\t界e\u{301}\u{1b}\r\nlast")
    );
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 4),
        focus: point(Side::New, 1, 5),
    };
    assert_eq!(
        selection.text(&document),
        None,
        "combining cluster cannot be bisected"
    );
}

#[test]
fn ending_and_empty_caret_are_explicit() {
    let document = DiffDocument::from_text("", "a\n");
    for (end, expected) in [(1, "a"), (2, "a\n"), (0, "")] {
        let selection = SourceSelection {
            anchor: point(Side::New, 1, 0),
            focus: point(Side::New, 1, end),
        };
        assert_eq!(selection.text(&document).as_deref(), Some(expected));
    }
}

#[test]
fn refuse_missing_context_and_side_crossings() {
    let document =
        DiffDocument::parse("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n@@ -3 +3 @@\n-c\n+d\n")
            .unwrap();
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 3, 1),
    };
    assert_eq!(selection.text(&document), None);
    let selection = SourceSelection {
        focus: point(Side::Old, 1, 1),
        ..selection
    };
    assert_eq!(selection.text(&document), None);
}

#[test]
fn keyboard_extends_anchor_and_stops_at_missing_context() {
    use ratatui_diff::{DiffState, SelectionMotion};
    let document = DiffDocument::from_text("", "界e\u{301}\n");
    let caret = point(Side::New, 1, 0);
    let mut state = DiffState::new();
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret
        }
    ));
    assert!(state.extend_selection(&document, SelectionMotion::Next));
    assert_eq!(state.selected_text(&document).as_deref(), Some("界"));
    assert!(state.extend_selection(&document, SelectionMotion::Next));
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("界e\u{301}")
    );
    assert!(state.extend_selection(&document, SelectionMotion::Next));
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("界e\u{301}\n")
    );
    assert!(!state.extend_selection(&document, SelectionMotion::Next));
    assert_eq!(state.selection().unwrap().anchor, caret);
    assert!(state.extend_selection(&document, SelectionMotion::Previous));
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("界e\u{301}")
    );
}

#[test]
fn layout_changes_keep_selection_and_replacement_clears_it() {
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState, ViewMode};
    let document = DiffDocument::from_text("", "selected source\n");
    let mut state = DiffState::new();
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 1, 15),
    };
    assert!(state.set_selection(&document, selection));
    for width in [80, 17, 49] {
        let area = Rect::new(3, 2, width, 20);
        let mut buffer = Buffer::empty(area);
        (&Diff::new(&document).mode(ViewMode::Split).wrap(true)).render(
            area,
            &mut buffer,
            &mut state,
        );
        assert_eq!(state.selection(), Some(selection));
        assert_eq!(
            state.selected_text(&document).as_deref(),
            Some("selected source")
        );
    }
    let clone = document.clone();
    let area = Rect::new(0, 0, 80, 20);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&clone)).render(area, &mut buffer, &mut state);
    assert_eq!(state.selection(), Some(selection));
    let replacement = DiffDocument::from_text("", "selected source\n");
    assert_eq!(state.selected_text(&replacement), None);
    (&Diff::new(&replacement)).render(area, &mut buffer, &mut state);
    assert_eq!(state.selection(), None);
}

#[test]
fn interleaved_diff_rows_extract_only_the_chosen_side() {
    let document = DiffDocument::from_text("keep\nold\ntail", "keep\nnew\ntail");
    for (side, expected) in [
        (Side::Old, "keep\nold\ntail"),
        (Side::New, "keep\nnew\ntail"),
    ] {
        let selection = SourceSelection {
            anchor: point(side, 1, 0),
            focus: point(side, 3, 4),
        };
        assert_eq!(selection.text(&document).as_deref(), Some(expected));
    }
}

#[test]
fn overlay_matches_source_hits_without_styling_padding_or_notation() {
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::style::Modifier;
    use ratatui_core::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState, DiffTheme, HitTest, ViewMode};
    let text = "\t界e\u{301}\u{1b} end";
    let document = DiffDocument::from_text("", text);
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 1, text.len()),
    };
    for mode in [ViewMode::Unified, ViewMode::Split] {
        for width in [17, 49, 50] {
            let area = Rect::new(3, 2, width, 20);
            let mut buffer = Buffer::empty(area);
            let mut state = DiffState::new();
            assert!(state.set_selection(&document, selection));
            let widget = Diff::new(&document)
                .mode(mode)
                .wrap(true)
                .whitespace(true)
                .theme(DiffTheme::monochrome());
            (&widget).render(area, &mut buffer, &mut state);
            let mut source_cells = 0;
            for y in area.y..area.bottom() {
                for x in area.x..area.right() {
                    let selected = matches!(
                        state.hit_test(x, y),
                        Some(HitTest::Source { new: Some(_), .. })
                    );
                    assert_eq!(
                        buffer[(x, y)].modifier.contains(Modifier::REVERSED),
                        selected,
                        "{mode:?} {width} ({x},{y})"
                    );
                    source_cells += usize::from(selected);
                }
            }
            assert!(source_cells > 0);
            assert_eq!(state.selected_text(&document).as_deref(), Some(text));
        }
    }
}

#[test]
fn context_selection_paints_only_its_split_side() {
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::style::Modifier;
    use ratatui_core::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState, HitTest, ViewMode};
    let document = DiffDocument::from_text("context\nold\n", "context\nnew\n");
    let mut state = DiffState::new();
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: point(Side::New, 1, 0),
            focus: point(Side::New, 1, 7)
        }
    ));
    let area = Rect::new(0, 0, 50, 12);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&document).mode(ViewMode::Split)).render(area, &mut buffer, &mut state);
    for y in 0..area.height {
        for x in 0..area.width {
            let expected = matches!(state.hit_test(x,y), Some(HitTest::Source { new: Some(range), .. }) if range.position.line == 1);
            assert_eq!(
                buffer[(x, y)].modifier.contains(Modifier::REVERSED),
                expected
            );
        }
    }
}

#[test]
fn line_motion_clamps_to_graphemes_and_cannot_cross_gaps() {
    use ratatui_diff::{DiffState, SelectionMotion};
    let document = DiffDocument::from_text("", "abcd\n界x\n");
    let mut state = DiffState::new();
    let caret = point(Side::New, 1, 2);
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret
        }
    ));
    assert!(state.extend_selection(&document, SelectionMotion::NextLine));
    assert_eq!(state.selection().unwrap().focus, point(Side::New, 2, 0));
    let document =
        DiffDocument::parse("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n@@ -3 +3 @@\n-c\n+d\n")
            .unwrap();
    let caret = point(Side::New, 1, 0);
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: caret,
            focus: caret
        }
    ));
    assert!(!state.extend_selection(&document, SelectionMotion::NextLine));
}

#[test]
fn custom_overlay_preserves_word_modifiers_and_source_mapping() {
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::style::{Color, Modifier, Style};
    use ratatui_core::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState, DiffTheme, HitTest};
    let document = DiffDocument::from_text("old\n", "new\n");
    let mut state = DiffState::new();
    let area = Rect::new(0, 0, 40, 8);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&document)
        .theme(DiffTheme::monochrome())
        .selection_style(Style::default().bg(Color::Blue));
    (&widget).render(area, &mut buffer, &mut state);
    let (x, y, range) = (0..area.height)
        .find_map(|y| {
            (0..area.width).find_map(|x| {
                if let Some(HitTest::Source {
                    new: Some(range), ..
                }) = state.hit_test(x, y)
                {
                    Some((x, y, range))
                } else {
                    None
                }
            })
        })
        .unwrap();
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: range.start_boundary(),
            focus: range.end_boundary()
        }
    ));
    (&widget).render(area, &mut buffer, &mut state);
    assert_eq!(buffer[(x, y)].bg, Color::Blue);
    assert!(buffer[(x, y)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(state.selected_text(&document).as_deref(), Some("n"));
    assert!(
        matches!(state.hit_test(x,y), Some(HitTest::Source { new: Some(after), .. }) if after == range)
    );
}

#[test]
fn explicit_focus_reveal_uses_graphemes_and_next_frame_geometry() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState, ViewMode};

    let document = DiffDocument::from_text("", "start\n0123456789界e\u{301}end\n\nlast\n");
    let mut state = DiffState::new();
    assert!(!state.reveal_selection());
    let selection = SourceSelection {
        anchor: point(Side::New, 1, 0),
        focus: point(Side::New, 2, 13),
    };
    let area = Rect::new(0, 0, 12, 3);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&document).line_numbers(false);
    (&widget).render(area, &mut buffer, &mut state);
    state.set_search(&document, "start", None);
    assert!(state.next_match());
    assert!(state.set_selection(&document, selection));
    assert!(state.reveal_selection());
    assert!(state.hit_test(1, 2).is_none());
    (&widget).render(area, &mut buffer, &mut state);
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == "e\u{301}")
    );
    assert!(state.horizontal_offset() > 0);
    assert_eq!(state.selection(), Some(selection));
    let revealed = state.horizontal_offset();
    (&widget).render(area, &mut buffer, &mut state);
    assert_eq!(
        state.horizontal_offset(),
        revealed,
        "search must not oscillate back"
    );
    state.scroll_horizontal(-100);
    (&widget).render(area, &mut buffer, &mut state);
    assert_eq!(
        state.horizontal_offset(),
        0,
        "manual navigation remains possible"
    );

    assert!(state.reveal_selection());
    let widget = widget.mode(ViewMode::Split).wrap(true);
    (&widget).render(area, &mut buffer, &mut state);
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == "e\u{301}")
    );
    assert_eq!(state.selection(), Some(selection));

    for byte in [17, 18] {
        let end = SourceSelection {
            focus: point(Side::New, 2, byte),
            ..selection
        };
        assert!(state.set_selection(&document, end));
        assert!(state.reveal_selection());
        (&widget).render(area, &mut buffer, &mut state);
        assert!(buffer.content().iter().any(|cell| cell.symbol() == "d"));
    }
    let blank = SourceSelection {
        focus: point(Side::New, 3, 0),
        ..selection
    };
    assert!(state.set_selection(&document, blank));
    assert!(state.reveal_selection());
    (&widget).render(area, &mut buffer, &mut state);
    assert!(
        (state.offset()..state.offset() + 3)
            .any(|row| state.source_at(row, Side::New).is_some_and(|p| p.line == 3))
    );
    for area in [Rect::new(0, 0, 0, 0), Rect::new(0, 0, 1, 1)] {
        assert!(state.reveal_selection());
        (&widget).render(area, &mut Buffer::empty(area), &mut state);
        assert_eq!(state.selection(), Some(blank));
    }
}

#[test]
fn validation_matches_extraction_and_rejection_is_atomic() {
    use ratatui_diff::DiffState;

    let documents = [
        DiffDocument::from_text("old\r\n", "界e\u{301}\r\n\nlast"),
        DiffDocument::parse("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n@@ -3 +3 @@\n-c\n+d\n")
            .unwrap(),
    ];
    for document in documents {
        let original = SourceSelection {
            anchor: point(Side::New, 1, 0),
            focus: point(Side::New, 1, 0),
        };
        let mut state = DiffState::new();
        assert!(state.set_selection(&document, original));
        for side in [Side::Old, Side::New] {
            for line in 0..=4 {
                for byte in 0..=12 {
                    let selection = SourceSelection {
                        anchor: original.anchor,
                        focus: point(side, line, byte),
                    };
                    let text = selection.text(&document);
                    let previous = state.selection();
                    assert_eq!(state.set_selection(&document, selection), text.is_some());
                    if let Some(text) = text {
                        assert_eq!(state.selected_text(&document), Some(text));
                        let reversed = SourceSelection {
                            anchor: selection.focus,
                            focus: selection.anchor,
                        };
                        assert!(state.set_selection(&document, reversed));
                    } else {
                        assert_eq!(state.selection(), previous);
                    }
                }
            }
        }
        let other_document = DiffDocument::from_text("", "replacement\n");
        let invalid = SourceSelection {
            focus: SourceBoundary {
                position: SourcePosition {
                    file: 1,
                    ..original.focus.position
                },
                ..original.focus
            },
            ..original
        };
        let previous = state.selection();
        let previous_text = state.selected_text(&document);
        assert!(!state.set_selection(&other_document, invalid));
        assert_eq!(state.selection(), previous);
        assert_eq!(state.selected_text(&document), previous_text);
        assert_eq!(state.selected_text(&other_document), None);
    }
}
