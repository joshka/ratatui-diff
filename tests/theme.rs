//! Rendered contrast and non-color cues for the opt-in RGB preset.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, HitTest, ViewMode};

#[test]
fn aardvark_ink_preserves_readable_roles_in_both_layouts() {
    let document = DiffDocument::from_text("same old value\n", "same new value\n");
    let theme = DiffTheme::aardvark_ink();
    for mode in [ViewMode::Unified, ViewMode::Split] {
        for width in [24, 80] {
            for whitespace in [false, true] {
                let area = Rect::new(0, 0, width, 20);
                let mut buffer = Buffer::empty(area);
                let mut state = DiffState::new();
                let diff = Diff::new(&document)
                    .mode(mode)
                    .wrap(true)
                    .whitespace(whitespace)
                    .theme(theme);
                (&diff).render(area, &mut buffer, &mut state);

                for (marker, line, word) in [
                    ("+", theme.insert, theme.insert_word),
                    ("-", theme.delete, theme.delete_word),
                ] {
                    assert!(buffer.content.iter().any(|cell| {
                        cell.symbol() == marker
                            && Some(cell.fg) == line.fg
                            && Some(cell.bg) == line.bg
                    }));
                    assert!(
                        buffer
                            .content
                            .iter()
                            .any(|cell| { Some(cell.fg) == line.fg && Some(cell.bg) == line.bg })
                    );
                    assert!(buffer.content.iter().any(|cell| {
                        Some(cell.fg) == line.fg
                            && Some(cell.bg) == word.bg
                            && cell.modifier.contains(Modifier::BOLD)
                    }));
                }
                assert!(buffer.content.iter().any(|cell| {
                    Some(cell.fg) == theme.header.fg && Some(cell.bg) == theme.header.bg
                }));
                if whitespace {
                    assert!(buffer.content.iter().any(|cell| {
                        cell.symbol() == "·" && !cell.modifier.contains(Modifier::BOLD)
                    }));
                }
                // Check the composed cell colors, including gutters over tinted row backgrounds.
                for cell in &buffer.content {
                    let minimum = if Some(cell.fg) == theme.gutter.fg {
                        3.0
                    } else {
                        4.5
                    };
                    if !matches!(cell.symbol(), "·" | "↪") {
                        assert!(contrast(cell.fg, cell.bg) >= minimum, "{cell:?}");
                    }
                }

                // Disabling word emphasis retains source-row colors and independent header counts.
                (&diff.word_highlights(false)).render(area, &mut buffer, &mut state);
                for (index, cell) in buffer.content.iter().enumerate() {
                    let x = area.x + (index % usize::from(area.width)) as u16;
                    let y = area.y + (index / usize::from(area.width)) as u16;
                    if matches!(state.hit_test(x, y), Some(HitTest::FileHeader { .. })) {
                        assert_eq!(Some(cell.bg), theme.header.bg);
                        continue;
                    }
                    if Some(cell.fg) == theme.insert.fg {
                        assert_eq!(Some(cell.bg), theme.insert.bg);
                        assert!(!cell.modifier.contains(Modifier::BOLD));
                    } else if Some(cell.fg) == theme.delete.fg {
                        assert_eq!(Some(cell.bg), theme.delete.bg);
                        assert!(!cell.modifier.contains(Modifier::BOLD));
                    }
                }

                // Reusing a buffer must clear RGB backgrounds and retain +/- and word cues.
                let mono = diff.theme(DiffTheme::monochrome());
                (&mono).render(area, &mut buffer, &mut state);
                assert!(
                    buffer
                        .content
                        .iter()
                        .all(|cell| { cell.fg == Color::Reset && cell.bg == Color::Reset })
                );
                for marker in ["+", "-"] {
                    assert!(buffer.content.iter().any(|cell| cell.symbol() == marker));
                }
                assert!(
                    buffer
                        .content
                        .iter()
                        .any(|cell| { cell.modifier.contains(Modifier::UNDERLINED) })
                );
            }
        }
    }
}

#[test]
fn split_row_background_reaches_the_edge_without_painting_outside_the_widget() {
    let document = DiffDocument::from_text("old\n", "new\nextra\n");
    let theme = DiffTheme::aardvark_ink();
    let outside = Style::default().bg(Color::Yellow);
    let mut state = DiffState::new();
    // Reuse state through odd/even resizing and a mode switch, as a host does.
    for mode in [ViewMode::Split, ViewMode::Unified, ViewMode::Split] {
        for width in [20, 21, 2, 1, 0] {
            let area = Rect::new(2, 1, width, 6);
            let mut buffer = Buffer::empty(Rect::new(0, 0, width + 4, 8));
            buffer.set_style(buffer.area, outside);
            let diff = Diff::new(&document).mode(mode).theme(theme);
            (&diff).render(area, &mut buffer, &mut state);
            for y in area.y..area.bottom() {
                assert_eq!(buffer[(area.right(), y)].bg, Color::Yellow);
                assert_eq!(buffer[(area.x - 1, y)].bg, Color::Yellow);
            }
            if mode != ViewMode::Split || width < 2 {
                continue;
            }
            assert_eq!(Some(buffer[(area.right() - 1, 3)].bg), theme.insert.bg);
            assert_eq!(Some(buffer[(area.right() - 1, 4)].bg), theme.insert.bg);
            assert_eq!(Some(buffer[(area.right() - 1, 5)].bg), theme.context.bg);
            let separator = area.x + (width - 1) / 2;
            assert_eq!(buffer[(separator, 3)].symbol(), "│");
            assert_eq!(Some(buffer[(separator, 3)].bg), theme.context.bg);
            // The unmatched addition has no old number or change marker.
            for x in area.x..separator {
                assert_eq!(buffer[(x, 4)].symbol(), " ");
                assert_eq!(Some(buffer[(x, 4)].bg), theme.context.bg);
            }
        }
    }
}

#[test]
fn phrase_spaces_keep_word_emphasis_when_markers_are_dimmed() {
    let document = DiffDocument::from_text("left one two end\n", "left three four end\n");
    for theme in [DiffTheme::aardvark_ink(), DiffTheme::monochrome()] {
        let area = Rect::new(0, 0, 40, 4);
        let mut buffer = Buffer::empty(area);
        let mut state = DiffState::new();
        let diff = Diff::new(&document)
            .line_numbers(false)
            .whitespace(true)
            .theme(theme);
        (&diff).render(area, &mut buffer, &mut state);
        for (x, y, line, word) in [
            (9, 2, theme.delete, theme.delete_word),
            (11, 3, theme.insert, theme.insert_word),
        ] {
            let cell = &buffer[(x, y)];
            assert_eq!(cell.symbol(), "·");
            if theme == DiffTheme::monochrome() {
                assert!(cell.modifier.contains(Modifier::DIM));
            } else {
                let expected = if y == 2 {
                    Color::Rgb(113, 60, 68)
                } else {
                    Color::Rgb(57, 105, 72)
                };
                assert_eq!(cell.fg, expected);
            }
            assert!(cell.modifier.contains(word.add_modifier));
            assert_eq!(cell.bg, line.patch(word).bg.unwrap_or(Color::Reset));
        }
    }
}

#[test]
fn change_markers_use_source_styles_independently_of_the_number_gutter() {
    let document = DiffDocument::from_text("old\n", "new\n");
    for mut theme in [
        DiffTheme::dark(),
        DiffTheme::light(),
        DiffTheme::aardvark_ink(),
        DiffTheme::monochrome(),
    ] {
        theme.gutter = Style::default()
            .fg(Color::Yellow)
            .bg(Color::Blue)
            .add_modifier(Modifier::DIM);
        for numbers in [true, false] {
            let area = Rect::new(0, 0, 30, 4);
            let mut buffer = Buffer::empty(area);
            let mut state = DiffState::new();
            let diff = Diff::new(&document).line_numbers(numbers).theme(theme);
            (&diff).render(area, &mut buffer, &mut state);
            for (symbol, y, row_style) in [("-", 2, theme.delete), ("+", 3, theme.insert)] {
                let marker_column = if numbers { 4 } else { 0 };
                let cell = &buffer[(marker_column, y)];
                assert_eq!(cell.symbol(), symbol);
                let expected = theme.context.patch(row_style);
                assert_eq!(cell.fg, expected.fg.unwrap_or(Color::Reset));
                assert_eq!(cell.bg, expected.bg.unwrap_or(Color::Reset));
                assert!(!cell.modifier.contains(Modifier::DIM));
            }
            if numbers {
                let number = &buffer[(0, 2)];
                assert_eq!(number.fg, Color::Yellow);
                assert_eq!(number.bg, Color::Blue);
                assert!(number.modifier.contains(Modifier::DIM));
            }
        }
    }
}

#[test]
fn rgb_continuation_cues_use_half_the_number_to_row_color_difference() {
    let document = DiffDocument::from_text("abcdefghij\n", "klmnopqrst\n");
    let area = Rect::new(0, 0, 19, 5);
    let mut buffer = Buffer::empty(area);
    let mut state = DiffState::new();
    let diff = Diff::new(&document)
        .mode(ViewMode::Split)
        .wrap(true)
        .theme(DiffTheme::aardvark_ink());
    (&diff).render(area, &mut buffer, &mut state);
    assert_eq!(buffer[(0, 2)].fg, Color::Rgb(111, 122, 143));
    let arrow = &buffer[(0, 3)];
    assert_eq!(arrow.symbol(), "↪");
    assert_eq!(arrow.bg, Color::Rgb(48, 32, 42));
    assert_eq!(arrow.fg, Color::Rgb(79, 77, 92));
    assert!(!arrow.modifier.contains(Modifier::DIM));
}

fn contrast(foreground: Color, background: Color) -> f64 {
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn luminance(color: Color) -> f64 {
    let Color::Rgb(red, green, blue) = color else {
        panic!("RGB preset left a cell dependent on the terminal palette: {color:?}");
    };
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}
