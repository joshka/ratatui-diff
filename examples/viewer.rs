//! Interactive host example; terminal and event ownership stay outside the library.
use std::error::Error;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, ViewMode};

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}
fn run(terminal: &mut DefaultTerminal) -> Result<(), Box<dyn Error>> {
    let old = "// A small greeting\nfn greeting(name: &str) -> String {\n    format!(\"Hello, {name}!\")\n}\n\nfn main() {\n    println!(\"{}\", greeting(\"world\"));\n}\n";
    let new = "// A friendly greeting\nfn greeting(name: &str) -> String {\n    format!(\"Welcome, {name}!\")\n}\n\nfn main() {\n    let name = \"Ratatui 🐭\";\n    println!(\"{}\", greeting(name));\n}\n";
    let document = DiffDocument::from_text(old, new);
    let mut state = DiffState::new();
    let mut mode = ViewMode::Unified;
    let mut wrap = false;
    let mut whitespace = false;
    let mut numbers = true;
    let mut theme = DiffTheme::dark();
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
            let widget = Diff::new(&document).mode(mode).wrap(wrap).whitespace(whitespace)
                .line_numbers(numbers).theme(theme);
            frame.render_stateful_widget(&widget, body, &mut state);
            let help = Line::raw("q quit | s split | w wrap | t spaces | n numbers | m mono | arrows/pgup/pgdn scroll");
            frame.render_widget(Paragraph::new(help), Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1));
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
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
                KeyCode::Char('w') => wrap = !wrap,
                KeyCode::Char('t') => whitespace = !whitespace,
                KeyCode::Char('n') => numbers = !numbers,
                KeyCode::Char('m') => {
                    theme = if theme == DiffTheme::dark() {
                        DiffTheme::monochrome()
                    } else {
                        DiffTheme::dark()
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
