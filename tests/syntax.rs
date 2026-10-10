//! Caller-facing bundled syntax preparation and rendering contracts.
#![cfg(feature = "syntax")]

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, FileSyntax, HitTest, Side, SourcePosition,
    SyntaxError, SyntaxHighlighter, SyntaxLimit, SyntaxLimits, SyntaxSource, SyntaxStyles,
    ViewMode,
};

fn prepare(document: &DiffDocument, old: &str, new: &str, theme: &str) -> SyntaxStyles {
    SyntaxHighlighter::bundled(theme)
        .unwrap()
        .prepare(
            document,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Full { old, new },
            }],
        )
        .unwrap()
}

fn draw(
    document: &DiffDocument,
    styles: Option<&SyntaxStyles>,
    state: &mut DiffState,
    width: u16,
    mode: ViewMode,
    wrap: bool,
) -> Buffer {
    let area = Rect::new(0, 0, width, 40);
    let mut buffer = Buffer::empty(area);
    let mut widget = Diff::new(document)
        .theme(DiffTheme::aardvark_ink())
        .mode(mode)
        .wrap(wrap);
    if let Some(styles) = styles {
        widget = widget.syntax_styles(styles).unwrap();
    }
    (&widget).render(area, &mut buffer, state);
    buffer
}

fn source_color(
    buffer: &Buffer,
    state: &DiffState,
    position: SourcePosition,
    byte: usize,
) -> Color {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if let Some(HitTest::Source { old, new }) = state.hit_test(x, y) {
                let range = if position.side == Side::Old { old } else { new };
                if range
                    .is_some_and(|range| range.position == position && range.bytes.contains(&byte))
                {
                    return buffer[(x, y)].fg;
                }
            }
        }
    }
    panic!("source not visible: {position:?}, byte {byte}")
}

#[test]
fn bundled_languages_themes_and_thread_traits() {
    fn thread_safe<T: Send + Sync>() {}
    thread_safe::<SyntaxHighlighter>();
    thread_safe::<SyntaxStyles>();
    thread_safe::<SyntaxError>();
    let highlighter = SyntaxHighlighter::bundled("Nord").unwrap();
    assert!(SyntaxHighlighter::themes().any(|name| name == "Nord"));
    assert!(highlighter.languages().any(|name| name == "Rust"));
    for language in ["rs", "Rust", "TOML", "TypeScript", "Dockerfile"] {
        let document = DiffDocument::from_text("", "value = 123\n");
        highlighter
            .prepare(
                &document,
                &[FileSyntax {
                    file: 0,
                    language,
                    source: SyntaxSource::Retained,
                }],
            )
            .unwrap();
    }
    assert!(matches!(
        SyntaxHighlighter::bundled("missing"),
        Err(SyntaxError::UnknownTheme(_))
    ));
}

#[test]
fn preparation_rejects_inputs_atomically_and_binds_document() {
    let old = "let café = 1;\r\n";
    let new = "let café = 2;";
    let document = DiffDocument::from_text(old, new);
    let highlighter = SyntaxHighlighter::bundled("Nord").unwrap();
    let input = FileSyntax {
        file: 0,
        language: "rs",
        source: SyntaxSource::Full { old, new },
    };
    let styles = highlighter.prepare(&document, &[input]).unwrap();
    let clone = document.clone();
    assert!(Diff::new(&clone).syntax_styles(&styles).is_ok());
    let replacement = DiffDocument::from_text(old, new);
    assert!(matches!(
        Diff::new(&replacement).syntax_styles(&styles),
        Err(SyntaxError::DocumentMismatch)
    ));
    assert!(matches!(
        highlighter.prepare(&document, &[input, input]),
        Err(SyntaxError::DuplicateFile(0))
    ));
    assert!(matches!(
        highlighter.prepare(&document, &[FileSyntax { file: 1, ..input }]),
        Err(SyntaxError::MissingFile(1))
    ));
    assert!(matches!(
        highlighter.prepare(
            &document,
            &[FileSyntax {
                language: "missing",
                ..input
            }]
        ),
        Err(SyntaxError::UnknownLanguage { .. })
    ));
    for mismatched in [
        "let café = 1;\n",
        "let café = 1;\r",
        "",
        "let café = 8;\r\n",
    ] {
        assert!(matches!(
            highlighter.prepare(
                &document,
                &[FileSyntax {
                    source: SyntaxSource::Full {
                        old: mismatched,
                        new
                    },
                    ..input
                }]
            ),
            Err(SyntaxError::SourceMismatch(_))
        ));
    }
    let empty = highlighter.prepare(&document, &[]).unwrap();
    assert!(Diff::new(&document).syntax_styles(&empty).is_ok());
}

#[test]
fn each_resource_limit_has_a_precise_failure() {
    let document = DiffDocument::from_text("", "let value = 10;\n");
    let input = FileSyntax {
        file: 0,
        language: "rs",
        source: SyntaxSource::Retained,
    };
    for (limits, expected) in [
        (
            SyntaxLimits {
                source_bytes: 0,
                ..SyntaxLimits::default()
            },
            SyntaxLimit::SourceBytes,
        ),
        (
            SyntaxLimits {
                line_bytes: 0,
                ..SyntaxLimits::default()
            },
            SyntaxLimit::LineBytes,
        ),
        (
            SyntaxLimits {
                spans: 0,
                ..SyntaxLimits::default()
            },
            SyntaxLimit::Spans,
        ),
    ] {
        let result = SyntaxHighlighter::bundled("Nord")
            .unwrap()
            .limits(limits)
            .prepare(&document, &[input]);
        assert!(
            matches!(result, Err(SyntaxError::LimitExceeded { limit, .. }) if limit == expected)
        );
    }
    // Full inputs count omitted source as well, even beyond the last retained line.
    let old = "let value = 1;\n";
    let new = "let value = 2;\n";
    let document = DiffDocument::from_text(old, new);
    let extra = format!("{new}{}", "x".repeat(100));
    let limits = SyntaxLimits {
        line_bytes: 20,
        ..SyntaxLimits::default()
    };
    let result = SyntaxHighlighter::bundled("Nord")
        .unwrap()
        .limits(limits)
        .prepare(
            &document,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Full { old, new: &extra },
            }],
        );
    assert!(matches!(
        result,
        Err(SyntaxError::LimitExceeded {
            limit: SyntaxLimit::LineBytes,
            ..
        })
    ));
}

#[test]
fn omitted_lexical_state_differs_from_explicit_retained_mode() {
    let old = "/* hidden opening\nlet value = 1;\n*/\n";
    let new = "/* hidden opening\nlet value = 2;\n*/\n";
    let document = DiffDocument::compare(old, new, 0);
    let full = prepare(&document, old, new, "Nord");
    let retained = SyntaxHighlighter::bundled("Nord")
        .unwrap()
        .prepare(
            &document,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Retained,
            }],
        )
        .unwrap();
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 2,
    };
    let mut full_state = DiffState::new();
    let full_buffer = draw(
        &document,
        Some(&full),
        &mut full_state,
        100,
        ViewMode::Unified,
        false,
    );
    let mut retained_state = DiffState::new();
    let retained_buffer = draw(
        &document,
        Some(&retained),
        &mut retained_state,
        100,
        ViewMode::Unified,
        false,
    );
    assert_ne!(
        source_color(&full_buffer, &full_state, position, 0),
        source_color(&retained_buffer, &retained_state, position, 0)
    );
}

#[test]
fn divergent_context_uses_independent_sides_and_new_unified_style() {
    let old = "/*\nlet value = 1;\n*/\n";
    let new = "//\nlet value = 1;\n//\n";
    let document = DiffDocument::compare(old, new, usize::MAX);
    let styles = prepare(&document, old, new, "Nord");
    let mut state = DiffState::new();
    let buffer = draw(
        &document,
        Some(&styles),
        &mut state,
        100,
        ViewMode::Split,
        false,
    );
    let new_position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 2,
    };
    let old_position = SourcePosition {
        side: Side::Old,
        ..new_position
    };
    let new_color = source_color(&buffer, &state, new_position, 0);
    assert_ne!(new_color, source_color(&buffer, &state, old_position, 0));
    let unified = draw(
        &document,
        Some(&styles),
        &mut state,
        100,
        ViewMode::Unified,
        false,
    );
    assert_eq!(source_color(&unified, &state, new_position, 0), new_color);
}

#[test]
fn styled_geometry_hits_and_extraction_match_plain_source() {
    let old = "";
    let new = "\tlet café = \"界é👩‍💻\";\r\n// controls: \u{1}\u{302}\nfinal";
    let document = DiffDocument::from_text(old, new);
    let styles = prepare(&document, old, new, "Nord");
    for mode in [ViewMode::Unified, ViewMode::Split] {
        for width in [15, 39, 100] {
            for wrap in [false, true] {
                let mut plain_state = DiffState::new();
                let mut styled_state = DiffState::new();
                let plain = draw(&document, None, &mut plain_state, width, mode, wrap);
                let styled = draw(
                    &document,
                    Some(&styles),
                    &mut styled_state,
                    width,
                    mode,
                    wrap,
                );
                assert_eq!(plain_state.row_count(), styled_state.row_count());
                for y in 0..40 {
                    for x in 0..width {
                        assert_eq!(plain[(x, y)].symbol(), styled[(x, y)].symbol());
                        assert_eq!(plain_state.hit_test(x, y), styled_state.hit_test(x, y));
                        if let Some(HitTest::Source { .. }) = styled_state.hit_test(x, y) {
                            assert_eq!(plain[(x, y)].bg, styled[(x, y)].bg);
                        } else {
                            assert_eq!(plain[(x, y)], styled[(x, y)]);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn overlays_preserve_syntax_foreground_until_explicitly_replaced() {
    let old = "let value = 1;\n";
    let new = "let value = 2;\n";
    let document = DiffDocument::from_text(old, new);
    let styles = prepare(&document, old, new, "Nord");
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 1,
    };
    let mut state = DiffState::new();
    let plain = draw(
        &document,
        Some(&styles),
        &mut state,
        80,
        ViewMode::Unified,
        false,
    );
    let syntax_fg = source_color(&plain, &state, position, 12);
    state.set_search(&document, "2", Some(Side::New));
    let searched = draw(
        &document,
        Some(&styles),
        &mut state,
        80,
        ViewMode::Unified,
        false,
    );
    assert_eq!(source_color(&searched, &state, position, 12), syntax_fg);
    assert!(
        searched
            .content
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::UNDERLINED))
    );
    let mut theme = DiffTheme::aardvark_ink();
    theme.insert_word = Style::default().fg(Color::Magenta);
    let area = Rect::new(0, 0, 80, 40);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&document)
        .theme(theme)
        .syntax_styles(&styles)
        .unwrap();
    (&widget).render(area, &mut buffer, &mut state);
    assert_eq!(source_color(&buffer, &state, position, 12), Color::Magenta);
}

#[test]
fn explicit_contrast_adapts_all_composed_surfaces_without_silent_conversion() {
    let old = include_str!("../examples/fixtures/syntax-old.rs");
    let new = include_str!("../examples/fixtures/syntax-new.rs");
    let document = DiffDocument::compare(old, new, usize::MAX);
    let raw = prepare(&document, old, new, "Coldark-Dark");
    let highlighter = SyntaxHighlighter::bundled("Coldark-Dark")
        .unwrap()
        .contrast_with(DiffTheme::aardvark_ink())
        .unwrap();
    let adapted = highlighter
        .prepare(
            &document,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Full { old, new },
            }],
        )
        .unwrap();
    let theme = DiffTheme::aardvark_ink();
    let insert = theme.context.patch(theme.insert);
    let delete = theme.context.patch(theme.delete);
    let backgrounds = [
        theme.context,
        insert,
        delete,
        insert.patch(theme.insert_word),
        delete.patch(theme.delete_word),
    ]
    .map(|style| match style.bg.unwrap() {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => panic!(),
    });
    let mut raw_state = DiffState::new();
    let raw_buffer = draw(
        &document,
        Some(&raw),
        &mut raw_state,
        160,
        ViewMode::Split,
        false,
    );
    let mut state = DiffState::new();
    let buffer = draw(
        &document,
        Some(&adapted),
        &mut state,
        160,
        ViewMode::Split,
        false,
    );
    let mut raw_minimum = f64::INFINITY;
    let mut adapted_minimum = f64::INFINITY;
    let mut adjusted = 0;
    for y in 0..40 {
        for x in 0..160 {
            if let Some(HitTest::Source { .. }) = state.hit_test(x, y) {
                assert_eq!(buffer[(x, y)].bg, raw_buffer[(x, y)].bg);
                assert_eq!(state.hit_test(x, y), raw_state.hit_test(x, y));
                let Color::Rgb(r, g, b) = buffer[(x, y)].fg else {
                    panic!()
                };
                let Color::Rgb(rr, rg, rb) = raw_buffer[(x, y)].fg else {
                    panic!()
                };
                let raw_passes = backgrounds
                    .iter()
                    .all(|&bg| contrast((rr, rg, rb), bg) >= 4.5);
                if raw_passes {
                    assert_eq!((r, g, b), (rr, rg, rb));
                }
                adjusted += usize::from((r, g, b) != (rr, rg, rb));
                for bg in backgrounds {
                    raw_minimum = raw_minimum.min(contrast((rr, rg, rb), bg));
                    adapted_minimum = adapted_minimum.min(contrast((r, g, b), bg));
                }
            }
        }
    }
    assert!(raw_minimum < 4.5);
    assert!(adapted_minimum >= 4.5);
    assert!(adjusted > 0);
    println!("Coldark-Dark/Aardvark Ink: raw={raw_minimum:.6}, adapted={adapted_minimum:.6}");
    assert!(matches!(
        SyntaxHighlighter::bundled("Nord")
            .unwrap()
            .contrast_with(DiffTheme::dark()),
        Err(SyntaxError::UnsupportedBackground { .. })
    ));
    let mut incompatible = DiffTheme::aardvark_ink();
    incompatible.context = incompatible.context.bg(Color::White);
    assert!(matches!(
        SyntaxHighlighter::bundled("Nord")
            .unwrap()
            .contrast_with(incompatible),
        Err(SyntaxError::UnsupportedBackground { .. })
    ));
    incompatible.context = incompatible.context.bg(Color::Rgb(255, 255, 255));
    assert!(matches!(
        SyntaxHighlighter::bundled("Nord")
            .unwrap()
            .contrast_with(incompatible),
        Err(SyntaxError::InsufficientContrast { .. })
    ));
}

#[test]
fn retained_parser_resets_at_gaps_and_hidden_source_preserves_state() {
    let patch = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n /*\n@@ -9 +9 @@\n let value = 1;\n";
    let document = DiffDocument::parse(patch).unwrap();
    let highlighter = SyntaxHighlighter::bundled("Nord").unwrap();
    let styles = highlighter
        .prepare(
            &document,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Retained,
            }],
        )
        .unwrap();
    let isolated = DiffDocument::from_text("", "let value = 1;\n");
    let isolated_styles = prepare(&isolated, "", "let value = 1;\n", "Nord");
    let mut state = DiffState::new();
    let mut isolated_state = DiffState::new();
    let buffer = draw(
        &document,
        Some(&styles),
        &mut state,
        100,
        ViewMode::Unified,
        false,
    );
    let other = draw(
        &isolated,
        Some(&isolated_styles),
        &mut isolated_state,
        100,
        ViewMode::Unified,
        false,
    );
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 9,
    };
    assert_eq!(
        source_color(&buffer, &state, position, 0),
        source_color(
            &other,
            &isolated_state,
            SourcePosition {
                line: 1,
                ..position
            },
            0
        )
    );

    let old = "/*\nlet value = 1;\n*/\n";
    let new = "/*\nlet value = 2;\n*/\n";
    let retained = DiffDocument::compare(old, new, usize::MAX);
    let full_styles = prepare(&retained, old, new, "Nord");
    let retained_styles = highlighter
        .prepare(
            &retained,
            &[FileSyntax {
                file: 0,
                language: "rs",
                source: SyntaxSource::Retained,
            }],
        )
        .unwrap();
    let area = Rect::new(0, 0, 100, 40);
    let mut state = DiffState::new();
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&retained)
        .context_lines(Some(0))
        .syntax_styles(&retained_styles)
        .unwrap();
    (&widget).render(area, &mut buffer, &mut state);
    for fold in state.context_folds().to_vec() {
        state.set_context_expanded(&fold, true);
    }
    (&widget).render(area, &mut buffer, &mut state);
    let expected = draw(
        &retained,
        Some(&full_styles),
        &mut DiffState::new(),
        100,
        ViewMode::Unified,
        false,
    );
    let position = SourcePosition {
        line: 2,
        ..position
    };
    let full_fg = source_color(
        &expected,
        &{
            let mut source_state = DiffState::new();
            draw(
                &retained,
                Some(&full_styles),
                &mut source_state,
                100,
                ViewMode::Unified,
                false,
            );
            source_state
        },
        position,
        0,
    );
    assert_eq!(source_color(&buffer, &state, position, 0), full_fg);
}

#[test]
fn replacement_palette_preserves_selection_search_folds_and_source_bytes() {
    use ratatui_diff::{SourceBoundary, SourceSelection};
    let new = "let café = \"界é👩‍💻\";\r\n// controls: \u{1}\u{302}\nfinal";
    let document = DiffDocument::from_text("", new);
    let styles = prepare(&document, "", new, "Nord");
    let alternate = prepare(&document, "", new, "Coldark-Dark");
    let mut state = DiffState::new();
    draw(
        &document,
        Some(&styles),
        &mut state,
        40,
        ViewMode::Split,
        true,
    );
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 1,
    };
    let selection = SourceSelection {
        anchor: SourceBoundary { position, byte: 0 },
        focus: SourceBoundary {
            position: SourcePosition {
                line: 3,
                ..position
            },
            byte: 5,
        },
    };
    assert!(state.set_selection(&document, selection));
    state.set_search(&document, "café", Some(Side::New));
    let fold = document.file_fold(0).unwrap();
    state.set_file_expanded(&fold, false);
    let rows = state.row_count();
    draw(
        &document,
        Some(&alternate),
        &mut state,
        40,
        ViewMode::Split,
        true,
    );
    assert_eq!(state.selection(), Some(selection));
    assert_eq!(state.selected_text(&document).as_deref(), Some(new));
    assert_eq!(state.search_matches().len(), 1);
    assert!(!state.file_expanded(&fold));
    state.set_file_expanded(&fold, true);
    draw(
        &document,
        Some(&alternate),
        &mut state,
        40,
        ViewMode::Split,
        true,
    );
    assert_eq!(state.row_count(), rows);
}

#[test]
fn grapheme_winner_and_synthetic_cells_do_not_change_source_contracts() {
    let document = DiffDocument::from_text("", "trué\t界\u{1}\u{302}");
    let styles = SyntaxHighlighter::bundled("Nord")
        .unwrap()
        .prepare(
            &document,
            &[FileSyntax {
                file: 0,
                language: "json",
                source: SyntaxSource::Retained,
            }],
        )
        .unwrap();
    let mut state = DiffState::new();
    let buffer = draw(
        &document,
        Some(&styles),
        &mut state,
        100,
        ViewMode::Unified,
        false,
    );
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 1,
    };
    assert_eq!(
        source_color(&buffer, &state, position, 0),
        source_color(&buffer, &state, position, 3)
    );
}

fn contrast(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
    fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
        fn channel(c: u8) -> f64 {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }
    let a = luminance(fg);
    let b = luminance(bg);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
