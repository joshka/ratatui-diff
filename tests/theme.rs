//! Rendered contrast and non-color cues for the opt-in RGB preset.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier};
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, ViewMode};

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
                            && Some(cell.fg) == theme.gutter.fg
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
                    assert!(buffer.content.iter().any(|cell| cell.symbol() == "·"));
                }
                // Check the composed cell colors, including gutters over tinted row backgrounds.
                for cell in &buffer.content {
                    assert!(contrast(cell.fg, cell.bg) >= 4.5, "{cell:?}");
                }

                // Disabling word emphasis must leave only the whole-line change colors.
                (&diff.word_highlights(false)).render(area, &mut buffer, &mut state);
                for cell in &buffer.content {
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
