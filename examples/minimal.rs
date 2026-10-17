//! Smallest complete host: compare two strings, draw the diff, and scroll it from the keyboard.
//!
//! The documentation site embeds this file as its getting-started example.
use std::error::Error;

use crossterm::event::{self, KeyCode};
use ratatui::DefaultTerminal;
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, ViewMode};

const OLD: &str = "\
fn retry_delay(status: u16) -> Option<u64> {
    let timeout_ms = 500;
    if status == 503 { Some(timeout_ms) } else { None }
}
";

const NEW: &str = "\
fn retry_delay(status: u16) -> Option<u64> {
    let timeout_ms = 1500;
    if matches!(status, 429 | 503) { Some(timeout_ms) } else { None }
}
";

fn main() -> Result<(), Box<dyn Error>> {
    // Compare once; the document is immutable and can be borrowed by every frame.
    let document = DiffDocument::from_text(OLD, NEW);
    ratatui::run(|terminal| run(terminal, &document))
}

fn run(terminal: &mut DefaultTerminal, document: &DiffDocument) -> Result<(), Box<dyn Error>> {
    // State keeps the viewport and cached layout between frames.
    let mut state = DiffState::new();
    let mut mode = ViewMode::Unified;
    loop {
        // Build the widget each frame; it only borrows the document.
        let diff = Diff::new(document)
            .mode(mode)
            .theme(DiffTheme::aardvark_ink());
        terminal.draw(|frame| frame.render_stateful_widget(&diff, frame.area(), &mut state))?;

        // The host owns key bindings; navigation methods take effect on the next draw.
        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('s') => {
                    mode = match mode {
                        ViewMode::Unified => ViewMode::Split,
                        _ => ViewMode::Unified,
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => state.scroll_lines(1),
                KeyCode::Char('k') | KeyCode::Up => state.scroll_lines(-1),
                KeyCode::PageDown => state.scroll_pages(1),
                KeyCode::PageUp => state.scroll_pages(-1),
                KeyCode::Char(']') => state.next_hunk(),
                KeyCode::Char('[') => state.previous_hunk(),
                _ => {}
            }
        }
    }
}
