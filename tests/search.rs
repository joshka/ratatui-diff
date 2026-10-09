//! Host-facing literal search and navigation contracts.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, Side, ViewMode};

fn draw(
    document: &DiffDocument,
    state: &mut DiffState,
    width: u16,
    mode: ViewMode,
    wrap: bool,
) -> Buffer {
    let area = Rect::new(0, 0, width, 4);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(document)
        .mode(mode)
        .wrap(wrap)
        .theme(DiffTheme::monochrome());
    (&widget).render(area, &mut buffer, state);
    buffer
}

fn active_text(buffer: &Buffer) -> String {
    buffer
        .content
        .iter()
        .filter(|c| c.modifier.contains(Modifier::REVERSED))
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn literal_utf8_nonoverlap_and_side_contracts() {
    let document = DiffDocument::compare("éé banana\nshared é\n", "é界 banana\nshared é\n", 3);
    let mut state = DiffState::new();
    state.set_search(&document, "é", None);
    let found = state.search_matches();
    assert_eq!(found.len(), 4);
    assert_eq!(found[0].bytes, 0..2);
    assert_eq!(found[1].bytes, 2..4);
    assert_eq!(found[2].position.side, Side::New);
    assert_eq!(found[3].position.line, 2);
    assert_eq!(found[3].bytes, 7..9);
    state.set_search(&document, "ana", Some(Side::Old));
    assert_eq!(state.search_matches().len(), 1);
    assert_eq!(state.search_matches()[0].bytes, 6..9);
    state.set_search(&document, "É", None);
    assert!(state.search_matches().is_empty());
    state.set_search(&document, "é\nshared", None);
    assert!(state.search_matches().is_empty());
    state.set_search(&document, "", None);
    assert!(!state.next_match());
    assert!(!state.previous_match());
}

#[test]
fn order_wraparound_updates_and_absent_context() {
    let patch = "--- a/first\n+++ b/first\n@@ -8,2 +8,2 @@\n common needle\n-old needle needle\n+new needle\n--- a/second\n+++ b/second\n@@ -20 +20 @@\n-old needle\n+new needle\n";
    let document = DiffDocument::parse(patch).unwrap();
    let mut state = DiffState::new();
    state.set_search(&document, "needle", None);
    assert_eq!(state.search_matches().len(), 6);
    assert_eq!(state.search_matches()[0].position.line, 8);
    assert_eq!(state.search_matches()[0].position.side, Side::New);
    assert_eq!(state.search_matches()[1].position.side, Side::Old);
    assert_eq!(state.search_matches()[2].bytes, 11..17);
    assert_eq!(state.search_matches()[4].position.file, 1);
    assert!(state.previous_match());
    assert_eq!(state.active_match(), Some(5));
    state.next_match();
    assert_eq!(state.active_match(), Some(0));
    state.set_search(&document, "needle", None);
    assert_eq!(state.active_match(), Some(0));
    state.set_search(&document, "needle", Some(Side::New));
    assert_eq!(state.active_match(), None);
    assert_eq!(state.search_matches().len(), 3);
    assert!(state.search_matches().iter().all(|m| m.position.line >= 8));
    state.set_search(&document, "first", None);
    assert!(state.search_matches().is_empty()); // Path headers are not source.
}

#[test]
fn navigation_reveals_offscreen_wrapped_and_horizontal_matches() {
    let new = format!("{}{}needle suffix\n", "line\n".repeat(30), "x".repeat(200));
    let document = DiffDocument::compare("", &new, 3);
    let mut state = DiffState::new();
    state.set_search(&document, "needle", None);
    draw(&document, &mut state, 40, ViewMode::Unified, false);
    assert!(state.next_match());
    let buffer = draw(&document, &mut state, 40, ViewMode::Unified, false);
    assert_eq!(active_text(&buffer), "needle");
    assert!(state.offset() > 20);
    assert!(state.horizontal_offset() > 150);
    let found = state.search_matches()[0].clone();
    for (width, mode) in [
        (23, ViewMode::Split),
        (17, ViewMode::Unified),
        (49, ViewMode::Split),
    ] {
        let buffer = draw(&document, &mut state, width, mode, true);
        assert!(active_text(&buffer).starts_with('n'));
        assert_eq!(state.search_matches()[0], found);
        assert_eq!(state.active_match(), Some(0));
        assert_eq!(state.horizontal_offset(), 0);
    }
}

#[test]
fn context_counts_once_and_side_selection_controls_painting() {
    let document = DiffDocument::compare("needle old\n", "needle new\n", 3);
    let mut state = DiffState::new();
    state.set_search(&document, "needle", Some(Side::Old));
    state.next_match();
    let buffer = draw(&document, &mut state, 49, ViewMode::Split, false);
    assert_eq!(active_text(&buffer), "needle");
    assert!(
        buffer
            .content
            .iter()
            .any(|c| c.modifier.contains(Modifier::UNDERLINED))
    );
    let context = DiffDocument::compare("shared needle\nold\n", "shared needle\nnew\n", 3);
    state.set_search(&context, "needle", None);
    assert_eq!(state.search_matches().len(), 1);
    state.next_match();
    let buffer = draw(&context, &mut state, 49, ViewMode::Split, false);
    assert_eq!(active_text(&buffer), "needleneedle");
    state.set_search(&context, "needle", Some(Side::Old));
    state.next_match();
    let buffer = draw(&context, &mut state, 49, ViewMode::Unified, false);
    assert_eq!(active_text(&buffer), "needle");
}

#[test]
fn frame_cache_and_document_replacement() {
    let document = DiffDocument::compare("", "needle\n", 3);
    let mut state = DiffState::new();
    state.set_search(&document, "needle", None);
    state.next_match();
    let results = state.search_matches().as_ptr();
    for _ in 0..10 {
        draw(&document, &mut state, 49, ViewMode::Split, true);
        assert_eq!(state.search_matches().as_ptr(), results);
        assert_eq!(state.active_match(), Some(0));
    }
    draw(&document.clone(), &mut state, 40, ViewMode::Unified, false);
    assert_eq!(state.search_matches().as_ptr(), results);
    let replacement = DiffDocument::compare("", "different needle\n", 3);
    draw(&replacement, &mut state, 40, ViewMode::Unified, false);
    assert!(state.search_matches().is_empty());
    assert!(state.search_query().is_empty());
    assert_eq!(state.active_match(), None);
    state.set_search(&replacement, "needle", None);
    state.next_match();
    let buffer = draw(&replacement, &mut state, 40, ViewMode::Unified, false);
    assert_eq!(active_text(&buffer), "needle");
    state.clear_search();
    assert!(state.search_query().is_empty());
}

#[test]
fn grapheme_tabs_controls_and_word_styles_compose_without_changing_source() {
    use ratatui_core::style::{Color, Style};

    let document = DiffDocument::compare("", "e\u{301}\t\u{1}word", 3);
    let mut state = DiffState::new();
    let area = Rect::new(0, 0, 40, 4);
    let theme = DiffTheme {
        insert: Style::default()
            .fg(Color::Green)
            .bg(Color::Rgb(0x16, 0x2b, 0x25)),
        search_active: Style::default().add_modifier(Modifier::REVERSED),
        ..DiffTheme::aardvark_ink()
    };
    for (query, bytes, display) in [
        ("\u{301}", 1..3, "e\u{301}"),
        ("\t", 3..4, "→  "),
        ("\u{1}", 4..5, "\\u{1}"),
    ] {
        state.set_search(&document, query, None);
        assert_eq!(state.search_matches()[0].bytes, bytes);
        state.next_match();
        let mut buffer = Buffer::empty(area);
        let widget = Diff::new(&document).whitespace(true).theme(theme);
        (&widget).render(area, &mut buffer, &mut state);
        assert_eq!(active_text(&buffer), display);
        assert!(
            buffer
                .content
                .iter()
                .filter(|c| c.modifier.contains(Modifier::REVERSED))
                .all(|c| c.bg == theme.insert.bg.unwrap())
        );
    }
    state.set_search(&document, "⏎", None);
    assert!(state.search_matches().is_empty());

    let document = DiffDocument::compare("old word\n", "new word\n", 3);
    state.set_search(&document, "new", None);
    state.next_match();
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&document).theme(theme)).render(area, &mut buffer, &mut state);
    let cells: Vec<_> = buffer
        .content
        .iter()
        .filter(|c| c.modifier.contains(Modifier::REVERSED))
        .collect();
    assert_eq!(cells.len(), 3);
    assert!(cells.iter().all(|c| c.modifier.contains(Modifier::BOLD)
        && c.fg == Color::Green
        && c.bg == theme.insert_word.bg.unwrap()));
}

#[test]
fn one_occurrence_can_paint_across_wrapped_rows() {
    let document = DiffDocument::compare("", "abcdefghijklmnop\n", 3);
    let mut state = DiffState::new();
    state.set_search(&document, "defghi", None);
    state.next_match();
    let area = Rect::new(0, 0, 10, 8);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&document)
        .wrap(true)
        .theme(DiffTheme::monochrome()))
        .render(area, &mut buffer, &mut state);
    assert_eq!(active_text(&buffer), "defghi");
    assert_eq!(state.search_matches()[0].bytes, 3..9);
}

#[test]
fn height_resize_reveals_active_match_and_tiny_areas_are_safe() {
    let document = DiffDocument::compare("", "a\nb\nc\nneedle\nd\ne\n", 3);
    let mut state = DiffState::new();
    state.set_search(&document, "needle", None);
    state.next_match();
    for (width, height) in [(40, 8), (40, 2), (0, 0), (1, 1), (40, 2)] {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        (&Diff::new(&document).theme(DiffTheme::monochrome())).render(
            area,
            &mut buffer,
            &mut state,
        );
        assert_eq!(state.active_match(), Some(0));
        if width == 40 {
            assert_eq!(active_text(&buffer), "needle");
        }
    }
}

#[test]
fn search_selection_and_hit_testing_share_source_ranges() {
    use ratatui_core::style::{Color, Style};
    use ratatui_diff::{HitTest, SourceBoundary, SourcePosition, SourceSelection};

    let document = DiffDocument::from_text("old\n", "needle\n");
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 1,
    };
    let mut state = DiffState::new();
    state.set_search(&document, "needle", Some(Side::New));
    assert!(state.next_match());
    assert!(state.set_selection(
        &document,
        SourceSelection {
            anchor: SourceBoundary { position, byte: 0 },
            focus: SourceBoundary { position, byte: 6 },
        }
    ));
    let area = Rect::new(0, 0, 80, 8);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&document).selection_style(Style::default().bg(Color::Blue));
    (&widget).render(area, &mut buffer, &mut state);
    let mut painted = 0;
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(HitTest::Source {
                new: Some(range), ..
            }) = state.hit_test(x, y)
            {
                assert_eq!(range.position, position);
                assert!(range.bytes.end <= 6);
                let cell = &buffer[(x, y)];
                assert_eq!(cell.bg, Color::Blue);
                assert!(cell.modifier.contains(Modifier::REVERSED));
                painted += 1;
            }
        }
    }
    assert_eq!(painted, 6);
    assert_eq!(state.selected_text(&document).as_deref(), Some("needle"));
    assert_eq!(state.active_match(), Some(0));
}
