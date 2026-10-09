//! Interactive host example; terminal and event ownership stay outside the library.
use std::error::Error;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
};
use crossterm::execute;
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, StatefulWidget};
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, HitTest, SelectionMotion, Side, SourceBoundary,
    SourcePosition, SourceSelection, ViewMode,
};

#[path = "viewer_fixtures/mod.rs"]
mod fixtures;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let mut arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    if arguments.as_slice() == ["--help"] {
        println!(
            "usage: viewer [--fixture NAME] [--aardvark-ink | --capture VARIANT | --measure VARIANT COLUMNS]\n\nFixtures: showcase (default), unicode, unicode-text, whitespace, multi-file\nCapture variants: unified, split, wrapped, whitespace, lines-only, no-numbers, mono"
        );
        return Ok(());
    }
    let mut fixture = "showcase";
    if let Some(index) = arguments
        .iter()
        .position(|argument| *argument == "--fixture")
    {
        fixture = *arguments
            .get(index + 1)
            .ok_or("--fixture requires a name")?;
        arguments.drain(index..=index + 1);
    }
    let document = fixtures::load(fixture)?;
    if let ["--capture", variant] = arguments.as_slice() {
        let widget = capture_widget(&document, variant)?;
        let mut terminal = ratatui::init();
        let result = capture(&mut terminal, &widget, &document, variant);
        ratatui::restore();
        return result;
    }
    if let ["--measure", variant, columns] = arguments.as_slice() {
        let widget = capture_widget(&document, variant)?;
        let area = Rect::new(0, 0, columns.parse()?, 1);
        let mut buffer = Buffer::empty(area);
        let mut state = DiffState::new();
        (&widget).render(area, &mut buffer, &mut state);
        println!("{}", state.row_count());
        return Ok(());
    }
    let color_theme =
        match arguments.as_slice() {
            [] => DiffTheme::dark(),
            ["--aardvark-ink"] => DiffTheme::aardvark_ink(),
            _ => return Err(
                "usage: viewer [--fixture NAME] [--aardvark-ink | --capture VARIANT | --measure VARIANT COLUMNS]"
                    .into(),
            ),
        };
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        run(&mut terminal, color_theme, document)
    })();
    let mouse_restore = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    mouse_restore?;
    result
}

/// Select the same presentation options used by the visual guide.
fn capture_widget<'a>(
    document: &'a DiffDocument,
    variant: &str,
) -> Result<Diff<'a>, Box<dyn Error>> {
    let widget = Diff::new(document).theme(DiffTheme::aardvark_ink());
    let split = widget.mode(ViewMode::Split);
    Ok(match variant {
        "unified" => widget,
        "split" => split,
        "lines-only" => split.word_highlights(false),
        "no-numbers" => split.line_numbers(false),
        "whitespace" => split.whitespace(true),
        "selection-wrapped" | "wrapped" => split.whitespace(true).wrap(true),
        "selection-mono" | "mono" => split
            .whitespace(true)
            .wrap(true)
            .theme(DiffTheme::monochrome()),
        _ => return Err(format!("unknown capture variant: {variant}").into()),
    })
}

/// Keep a stable widget-only frame available until the capture runner sends q.
fn capture(
    terminal: &mut DefaultTerminal,
    widget: &Diff<'_>,
    document: &DiffDocument,
    variant: &str,
) -> Result<(), Box<dyn Error>> {
    let mut state = DiffState::new();
    if variant.starts_with("selection-") {
        let anchor = SourceBoundary {
            position: SourcePosition {
                file: 0,
                side: Side::New,
                line: 4,
            },
            byte: 8,
        };
        let focus = SourceBoundary {
            position: SourcePosition {
                line: 6,
                ..anchor.position
            },
            byte: 22,
        };
        assert!(state.set_selection(document, SourceSelection { anchor, focus }));
    }
    terminal.draw(|frame| frame.render_stateful_widget(widget, frame.area(), &mut state))?;
    loop {
        if let Event::Key(key) = event::read()?
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            return Ok(());
        }
    }
}

fn run(
    terminal: &mut DefaultTerminal,
    color_theme: DiffTheme,
    document: DiffDocument,
) -> Result<(), Box<dyn Error>> {
    let mut state = DiffState::new();
    let mut mode = ViewMode::Unified;
    let mut wrap = false;
    let mut whitespace = false;
    let mut numbers = true;
    let mut words = true;
    let mut theme = color_theme;
    let mut pointer = None;
    let mut copy_preview = None;
    let mut body_area = Rect::default();
    let mut query = String::new();
    let mut editing_search = false;
    loop {
        terminal.draw(|frame| {
            // Outer spacing belongs to the host; the diff fills its supplied rectangle.
            frame.render_widget(Block::default().style(theme.context), frame.area());
            let area = frame.area().inner(Margin::new(2, 1));
            let [title_area, _, body, feedback_area, help_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);
            let name = if theme == DiffTheme::monochrome() {
                "Monochrome"
            } else if theme == DiffTheme::aardvark_ink() {
                "Aardvark Ink"
            } else {
                "Terminal palette"
            };
            let mut title = format!("{mode:?} · {name}");
            if area.width >= 70 {
                title.push_str(if whitespace {
                    " · whitespace on"
                } else {
                    " · whitespace off"
                });
            }
            if editing_search || !state.search_query().is_empty() {
                let current = state.active_match().map_or(0, |n| n + 1);
                title = format!(
                    "/{query} · {current}/{} · {mode:?} · {name}{}",
                    state.search_matches().len(),
                    if editing_search {
                        " · Enter to find"
                    } else {
                        ""
                    }
                );
            }
            frame.render_widget(Paragraph::new(title), title_area);
            body_area = body;
            let widget = Diff::new(&document)
                .mode(mode)
                .wrap(wrap)
                .whitespace(whitespace)
                .line_numbers(numbers)
                .word_highlights(words)
                .theme(theme);
            frame.render_stateful_widget(&widget, body, &mut state);
            if let Some(ref text) = copy_preview {
                frame.render_widget(
                    Paragraph::new(format!("Copy preview: {text:?}")),
                    feedback_area,
                );
            } else if let Some((x, y)) = pointer {
                let feedback = match state.hit_test(x, y) {
                    Some(HitTest::Source { old, new }) => {
                        let range = new.or(old).expect("source hit has a side");
                        format!(
                            "Pointer ({x},{y}): file {} {:?} line {} bytes {}..{}",
                            range.position.file,
                            range.position.side,
                            range.position.line,
                            range.bytes.start,
                            range.bytes.end
                        )
                    }
                    hit => format!("Pointer ({x},{y}): {hit:?}"),
                };
                frame.render_widget(Paragraph::new(feedback), feedback_area);
            }
            let help = if area.width >= 70 {
                "/ search · f/F match · v select · c copy · s split · w wrap · q quit"
            } else {
                "/ search · f/F match · v select · c copy"
            };
            frame.render_widget(Paragraph::new(Line::raw(help)), help_area);
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        if let Event::Mouse(mouse) = event {
            pointer = Some((mouse.column, mouse.row));
            update_pointer_selection(&document, &mut state, mouse.column, mouse.row, mouse.kind);
            copy_preview = None;
        }
        if let Event::Key(key) = event {
            if editing_search {
                match key.code {
                    KeyCode::Enter => {
                        editing_search = false;
                        state.next_match();
                    }
                    KeyCode::Esc => {
                        editing_search = false;
                        query.clear();
                        state.clear_search();
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        state.set_search(&document, &query, None);
                    }
                    KeyCode::Char(c) => {
                        query.push(c);
                        state.set_search(&document, &query, None);
                    }
                    _ => {}
                }
                continue;
            }
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Esc if state.selection().is_some() => {
                    state.clear_selection();
                    copy_preview = None;
                }
                KeyCode::Char('v') => {
                    let hit = pointer.and_then(|(x, y)| state.hit_test(x, y));
                    let caret = if let Some(HitTest::Source { old, new }) = hit {
                        new.or(old).map(|range| range.start_boundary())
                    } else {
                        (state.offset()..state.row_count())
                            .find_map(|row| state.source_at(row, Side::New))
                            .map(|position| SourceBoundary { position, byte: 0 })
                    };
                    if let Some(caret) = caret {
                        state.set_selection(
                            &document,
                            SourceSelection {
                                anchor: caret,
                                focus: caret,
                            },
                        );
                    }
                    copy_preview = None;
                }
                KeyCode::Char('c') => copy_preview = state.selected_text(&document),
                KeyCode::Right if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::Next);
                    copy_preview = None;
                }
                KeyCode::Left if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::Previous);
                    copy_preview = None;
                }
                KeyCode::Home if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::LineStart);
                    copy_preview = None;
                }
                KeyCode::End if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::LineEnd);
                    copy_preview = None;
                }
                KeyCode::Down if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::NextLine);
                    copy_preview = None;
                }
                KeyCode::Up if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::PreviousLine);
                    copy_preview = None;
                }
                KeyCode::Char('/') => {
                    editing_search = true;
                    query.clear();
                    state.clear_search();
                }
                KeyCode::Char('f') => {
                    state.next_match();
                }
                KeyCode::Char('F') => {
                    state.previous_match();
                }
                KeyCode::Esc if !query.is_empty() => {
                    query.clear();
                    state.clear_search();
                }
                KeyCode::Esc => break,
                KeyCode::Down | KeyCode::Char('j') => state.scroll_lines(1),
                KeyCode::Up | KeyCode::Char('k') => state.scroll_lines(-1),
                KeyCode::Left => state.scroll_horizontal(-4),
                KeyCode::Right => state.scroll_horizontal(4),
                KeyCode::PageDown => state.scroll_pages(1),
                KeyCode::PageUp => state.scroll_pages(-1),
                KeyCode::Home => state.start(),
                KeyCode::End => state.end(),
                KeyCode::Char(']') => state.next_hunk(),
                KeyCode::Char('[') => state.previous_hunk(),
                KeyCode::Char('s') => {
                    mode = if mode == ViewMode::Unified {
                        ViewMode::Split
                    } else {
                        ViewMode::Unified
                    }
                }
                KeyCode::Char('p') => {
                    pointer = Some((
                        body_area.x.saturating_add(15),
                        body_area.y.saturating_add(5),
                    ))
                }
                KeyCode::Char('w') => wrap = !wrap,
                KeyCode::Char('t') => whitespace = !whitespace,
                KeyCode::Char('n') => numbers = !numbers,
                KeyCode::Char('i') => words = !words,
                KeyCode::Char('m') => {
                    theme = if theme == color_theme {
                        DiffTheme::monochrome()
                    } else {
                        color_theme
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Host drag policy: start at the leading edge, extend to the hit grapheme's trailing edge.
fn update_pointer_selection(
    document: &DiffDocument,
    state: &mut DiffState,
    x: u16,
    y: u16,
    kind: MouseEventKind,
) {
    let Some(HitTest::Source { old, new }) = state.hit_test(x, y) else {
        return;
    };
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(range) = new.or(old) {
                state.set_selection(
                    document,
                    SourceSelection {
                        anchor: range.start_boundary(),
                        focus: range.end_boundary(),
                    },
                );
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(mut selection) = state.selection() {
                let range = match selection.anchor.position.side {
                    Side::Old => old,
                    Side::New => new,
                };
                if let Some(range) = range {
                    selection.focus = range.end_boundary();
                    state.set_selection(document, selection);
                }
            }
        }
        _ => {}
    }
}
