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
    Diff, DiffDocument, DiffState, DiffTheme, FileFold, HitTest, SelectionMotion, Side,
    SourceBoundary, SourcePosition, SourceSelection, ViewMode,
};

#[path = "viewer_fixtures/mod.rs"]
mod fixtures;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let mut arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    #[cfg(feature = "syntax")]
    if arguments.as_slice() == ["--syntax-licenses"] {
        print!("{}", ratatui_diff::SyntaxHighlighter::acknowledgements());
        return Ok(());
    }
    if arguments.as_slice() == ["--help"] {
        println!(
            "usage: viewer [--fixture NAME] [--aardvark-ink | --capture VARIANT | --measure VARIANT COLUMNS]\n\nUse --sticky-headers to keep the current file header visible.\n\nFixtures: showcase (default), context, files, alignment, unicode, unicode-text, whitespace, multi-file\nCapture variants: unified, split, wrapped, whitespace, lines-only, no-numbers, mono"
        );
        #[cfg(feature = "syntax")]
        println!("Syntax: --syntax THEME [--syntax-contrast], --syntax-licenses; h toggles prepared syntax.
Use --fixture syntax for complete Rust sources; other fixtures use best-effort retained input.");
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
    #[cfg(feature = "syntax")]
    let syntax_contrast = if let Some(index) = arguments
        .iter()
        .position(|argument| *argument == "--syntax-contrast")
    {
        arguments.remove(index);
        true
    } else {
        false
    };
    #[cfg(feature = "syntax")]
    let syntax = if let Some(index) = arguments
        .iter()
        .position(|argument| *argument == "--syntax")
    {
        let theme = *arguments
            .get(index + 1)
            .ok_or("--syntax requires a bundled theme name")?;
        arguments.drain(index..=index + 1);
        Some(prepare_syntax(&document, fixture, theme, syntax_contrast)?)
    } else {
        None
    };
    if let ["--capture", variant] = arguments.as_slice() {
        let widget = capture_widget(&document, variant)?;
        #[cfg(feature = "syntax")]
        let widget = if let Some(styles) = &syntax {
            widget.syntax_styles(styles)?
        } else {
            widget
        };
        let mut terminal = ratatui::init();
        let result = capture(&mut terminal, &widget, &document, variant);
        ratatui::restore();
        return result;
    }
    if let ["--measure", variant, columns] = arguments.as_slice() {
        let widget = capture_widget(&document, variant)?;
        #[cfg(feature = "syntax")]
        let widget = if let Some(styles) = &syntax {
            widget.syntax_styles(styles)?
        } else {
            widget
        };
        let area = Rect::new(0, 0, columns.parse()?, 1);
        let mut buffer = Buffer::empty(area);
        let mut state = DiffState::new();
        (&widget).render(area, &mut buffer, &mut state);
        println!("{}", state.row_count());
        return Ok(());
    }
    let sticky_headers = arguments.contains(&"--sticky-headers");
    arguments.retain(|argument| *argument != "--sticky-headers");
    let color_theme =
        match arguments.as_slice() {
            [] => DiffTheme::dark(),
            ["--aardvark-ink"] => DiffTheme::aardvark_ink(),
            _ => return Err(
                "usage: viewer [--fixture NAME] [--aardvark-ink | --capture VARIANT | --measure VARIANT COLUMNS]"
                    .into(),
            ),
        };
    #[cfg(feature = "syntax")]
    if syntax_contrast && color_theme != DiffTheme::aardvark_ink() {
        return Err("--syntax-contrast requires --aardvark-ink for interactive viewing".into());
    }
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        run(
            &mut terminal,
            color_theme,
            document,
            sticky_headers,
            #[cfg(feature = "syntax")]
            syntax,
        )
    })();
    let mouse_restore = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    mouse_restore?;
    result
}

/// Highlight source once; the feature-enabled host never tokenizes or maps spans itself.
#[cfg(feature = "syntax")]
fn prepare_syntax(
    document: &DiffDocument,
    fixture: &str,
    theme: &str,
    contrast: bool,
) -> Result<ratatui_diff::SyntaxStyles, Box<dyn Error>> {
    use ratatui_diff::{FileSyntax, SyntaxHighlighter, SyntaxSource};
    let highlighter = SyntaxHighlighter::bundled(theme)?;
    let highlighter = if contrast {
        highlighter.contrast_with(DiffTheme::aardvark_ink())?
    } else {
        highlighter
    };
    let files: Vec<_> = document
        .files()
        .iter()
        .enumerate()
        .filter(|(_, file)| !file.hunks.is_empty())
        .map(|(file, _)| FileSyntax {
            file,
            language: if fixture == "showcase" || fixture == "syntax" {
                "rs"
            } else {
                "txt"
            },
            source: if fixture == "syntax" {
                let (old, new) = fixtures::syntax_sources();
                SyntaxSource::Full { old, new }
            } else {
                SyntaxSource::Retained
            },
        })
        .collect();
    Ok(highlighter.prepare(document, &files)?)
}

/// Select the same presentation options used by the visual guide.
fn capture_widget<'a>(
    document: &'a DiffDocument,
    variant: &str,
) -> Result<Diff<'a>, Box<dyn Error>> {
    let widget = Diff::new(document).theme(DiffTheme::aardvark_ink());
    let split = widget.mode(ViewMode::Split);
    Ok(match variant {
        "files-before" | "files-expanded" | "files-collapsed" => widget.context_lines(Some(3)),
        "files-collapsed-stats" => widget.context_lines(Some(3)).show_stats(true),
        "files-expanded-split" => split.context_lines(Some(3)).wrap(true),
        "files-collapsed-split" => split.context_lines(Some(3)),
        "files-collapsed-mono" => split.context_lines(Some(3)).theme(DiffTheme::monochrome()),
        "context-before" => widget,
        "context-narrow-before" => split.wrap(true),
        "context-split-before" => split,
        "context-gap" => widget.context_lines(Some(3)),
        "context-unified" => widget.context_lines(Some(3)),
        "context-split" => split.context_lines(Some(3)),
        "context-mono" => split
            .context_lines(Some(3))
            .theme(DiffTheme::monochrome())
            .wrap(true),
        "context-wrapped" => split.context_lines(Some(3)).wrap(true),
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
    if variant.starts_with("files-collapsed") {
        for index in 0..document.files().len() {
            let fold = document.file_fold(index).expect("validated file index");
            state.set_file_expanded(&fold, false);
        }
    }
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
    if variant == "context-gap" {
        state.next_file();
        terminal.draw(|frame| frame.render_stateful_widget(widget, frame.area(), &mut state))?;
    }
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
    sticky_headers: bool,
    #[cfg(feature = "syntax")] syntax: Option<ratatui_diff::SyntaxStyles>,
) -> Result<(), Box<dyn Error>> {
    let mut state = DiffState::new();
    #[cfg(feature = "syntax")]
    let mut syntax_enabled = syntax.is_some();
    let mut mode = ViewMode::Unified;
    let mut wrap = false;
    let mut whitespace = false;
    let mut numbers = true;
    let mut words = true;
    let mut show_stats = false;
    let mut theme = color_theme;
    let mut pointer = None;
    let mut copy_preview = None;
    let mut body_area = Rect::default();
    let mut query = String::new();
    let mut editing_search = false;
    let mut focused_file = None;
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
            let wrap_label = if wrap { "wrapped" } else { "unwrapped" };
            let mut title = format!("{mode:?} · {wrap_label} · {name}");
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
            #[cfg(feature = "syntax")]
            if syntax.is_some() && area.width >= 70 {
                title.push_str(if syntax_enabled && theme != DiffTheme::monochrome() { " · syntax on (h)" } else { " · syntax off (h)" });
            }
            frame.render_widget(Paragraph::new(title), title_area);
            body_area = body;
            let widget = Diff::new(&document)
                .context_lines(Some(3))
                .show_stats(show_stats)
                .sticky_file_headers(sticky_headers)
                .mode(mode)
                .wrap(wrap)
                .whitespace(whitespace)
                .line_numbers(numbers)
                .word_highlights(words)
                .theme(theme);
            #[cfg(feature = "syntax")]
            let widget = if syntax_enabled && theme != DiffTheme::monochrome() {
                syntax.as_ref().map_or(widget, |styles| widget.syntax_styles(styles).expect("prepared for this document"))
            } else { widget };
            frame.render_stateful_widget(&widget, body, &mut state);
            focused_file = focused_file.filter(|&fold| (body.y..body.bottom()).any(|y| {
                matches!(state.hit_test(body.x,y),Some(HitTest::FileHeader{fold:visible}) if visible == fold)
            }));
            if let Some(ref text) = copy_preview {
                frame.render_widget(
                    Paragraph::new(format!("Text preview: {text:?}")),
                    feedback_area,
                );
            } else if let Some((x, y)) = pointer {
                let feedback = match state.hit_test(x, y) {
                    Some(HitTest::FileHeader {fold}) => {
                        let status = if state.file_expanded(&fold) { "expanded" } else { "collapsed" };
                        format!("File {} · {status} · click to toggle", fold.file()+1)
                    }
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
            } else if let Some(selection) = state.selection() {
                let focus = selection.focus;
                frame.render_widget(
                    Paragraph::new(format!(
                        "Selection · {:?} line {} · byte {}",
                        focus.position.side, focus.position.line, focus.byte
                    )),
                    feedback_area,
                );
            }
            if let Some(fold) = focused_file {
                let file = &document.files()[fold.file()];
                let path = file.new_path.as_deref().or(file.old_path.as_deref()).unwrap_or("");
                frame.render_widget(Paragraph::new(format!("File focus · {path} · Enter toggle")),feedback_area);
            }
            let help = interaction_hint(
                editing_search,
                state.selection().is_some(),
                !query.is_empty(),
                area.width,
            );
            frame.render_widget(Paragraph::new(Line::raw(help)), help_area);
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        if let Event::Mouse(mouse) = event {
            focused_file = None;
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
            if matches!(key.code, KeyCode::Char('s' | 'w' | 't' | 'n')) {
                state.reveal_selection();
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
                    state.reveal_selection();
                    copy_preview = None;
                }
                KeyCode::Left if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::Previous);
                    state.reveal_selection();
                    copy_preview = None;
                }
                KeyCode::Home if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::LineStart);
                    state.reveal_selection();
                    copy_preview = None;
                }
                KeyCode::End if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::LineEnd);
                    state.reveal_selection();
                    copy_preview = None;
                }
                KeyCode::Down if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::NextLine);
                    state.reveal_selection();
                    copy_preview = None;
                }
                KeyCode::Up if state.selection().is_some() => {
                    state.extend_selection(&document, SelectionMotion::PreviousLine);
                    state.reveal_selection();
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
                KeyCode::Esc => {
                    copy_preview = None;
                    pointer = None;
                }
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
                KeyCode::Tab | KeyCode::BackTab => {
                    focused_file = next_visible_file(
                        &state,
                        body_area,
                        focused_file,
                        key.code == KeyCode::Tab,
                    );
                    pointer = None;
                }
                KeyCode::Enter => {
                    if let Some(fold) =
                        focused_file.or_else(|| current_file(&state, pointer, body_area))
                    {
                        state.set_file_expanded(&fold, !state.file_expanded(&fold));
                    }
                }
                KeyCode::Char('A') => set_all_files(&mut state, true),
                KeyCode::Char('Z') => set_all_files(&mut state, false),
                KeyCode::Char('e') => {
                    let fold = (body_area.y..body_area.bottom()).find_map(|y| {
                        if let Some(HitTest::Fold { fold }) = state.hit_test(body_area.x, y) {
                            Some(fold)
                        } else {
                            None
                        }
                    });
                    if let Some(fold) = fold {
                        state.set_context_expanded(&fold, true);
                    }
                }
                KeyCode::Char('z') => {
                    for fold in state.context_folds().to_vec() {
                        state.set_context_expanded(&fold, false);
                    }
                }
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
                #[cfg(feature = "syntax")]
                KeyCode::Char('h') => syntax_enabled = !syntax_enabled,
                KeyCode::Char('d') => {
                    show_stats = !show_stats;
                    pointer = None;
                }
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

/// Keyboard focus belongs to the host, independently of final-viewport clamping.
fn next_visible_file(
    state: &DiffState,
    area: Rect,
    focused: Option<FileFold>,
    forward: bool,
) -> Option<FileFold> {
    let files: Vec<_> = (area.y..area.bottom())
        .filter_map(|y| {
            if let Some(HitTest::FileHeader { fold }) = state.hit_test(area.x, y) {
                Some(fold)
            } else {
                None
            }
        })
        .collect();
    if files.is_empty() {
        return None;
    }
    let index = files.iter().position(|&fold| Some(fold) == focused);
    let next = match (index, forward) {
        (None, true) => 0,
        (None, false) => files.len() - 1,
        (Some(index), true) => (index + 1) % files.len(),
        (Some(index), false) => (index + files.len() - 1) % files.len(),
    };
    Some(files[next])
}

/// File controls are independent from the example's e/z retained-context controls.
fn current_file(state: &DiffState, pointer: Option<(u16, u16)>, area: Rect) -> Option<FileFold> {
    if let Some((x, y)) = pointer
        && let Some(HitTest::FileHeader { fold }) = state.hit_test(x, y)
    {
        return Some(fold);
    }
    let file = match state.hit_test(area.x, area.y)? {
        HitTest::FileHeader { fold } => return Some(fold),
        HitTest::Header { file } => file,
        HitTest::Fold { fold } => fold.file,
        _ => {
            state
                .source_at(state.offset(), Side::New)
                .or_else(|| state.source_at(state.offset(), Side::Old))?
                .file
        }
    };
    state.file_folds().get(file).copied()
}

fn set_all_files(state: &mut DiffState, expanded: bool) {
    for fold in state.file_folds().to_vec() {
        state.set_file_expanded(&fold, expanded);
    }
}

/// Host drag policy: start at the leading edge, extend to the hit grapheme's trailing edge.
fn update_pointer_selection(
    document: &DiffDocument,
    state: &mut DiffState,
    x: u16,
    y: u16,
    kind: MouseEventKind,
) {
    if kind == MouseEventKind::Down(MouseButton::Left)
        && let Some(HitTest::FileHeader { fold }) = state.hit_test(x, y)
    {
        state.set_file_expanded(&fold, !state.file_expanded(&fold));
        return;
    }
    if kind == MouseEventKind::Down(MouseButton::Left)
        && let Some(HitTest::Fold { fold }) = state.hit_test(x, y)
    {
        state.set_context_expanded(&fold, true);
        return;
    }
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

/// Keep the active input policy visible before secondary presentation controls.
fn interaction_hint(editing: bool, selecting: bool, searching: bool, width: u16) -> &'static str {
    if editing {
        if width < 45 {
            "Enter find · Esc cancel"
        } else {
            "Type literal search · Enter find · Esc cancel"
        }
    } else if selecting {
        if width < 78 {
            "Arrows select · c preview · Esc clear"
        } else {
            "←/→ grapheme · ↑/↓ source line · Home/End · c preview · Esc clear selection"
        }
    } else if searching {
        if width < 45 {
            "f/F match · v select · Esc clear"
        } else {
            "f/F match · v select · / search · Esc clear search · q quit"
        }
    } else if width < 45 {
        "Tab file · Enter toggle · A/Z all"
    } else {
        "Tab file · Enter toggle · A/Z all · d totals · e/z context · / find · q quit"
    }
}

#[cfg(test)]
mod tests {
    use super::interaction_hint;

    #[test]
    fn hints_prioritize_active_input_policy_and_label_preview() {
        assert_eq!(
            interaction_hint(true, true, true, 40),
            "Enter find · Esc cancel"
        );
        assert_eq!(
            interaction_hint(false, true, true, 40),
            "Arrows select · c preview · Esc clear"
        );
        assert_eq!(
            interaction_hint(false, false, true, 40),
            "f/F match · v select · Esc clear"
        );
        assert!(interaction_hint(false, true, true, 100).contains("source line"));
        assert!(interaction_hint(false, false, false, 100).contains("q quit"));
    }
}
