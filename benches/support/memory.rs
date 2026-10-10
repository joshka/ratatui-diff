use std::mem::size_of;
use std::process::Command;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffLine, DiffState, ViewMode};

pub fn run() {
    let (old, new) = super::scenarios::context_sources(100_000);
    report("input", None);
    println!("input_capacity_bytes={}", old.capacity() + new.capacity());
    let document = DiffDocument::compare(&old, &new, usize::MAX);
    report("prepared_with_inputs", None);
    let mut text = 0;
    let mut lines = 0;
    let mut highlights = 0;
    for file in document.files() {
        for hunk in &file.hunks {
            lines += hunk.lines.capacity() * size_of::<DiffLine>();
            for line in &hunk.lines {
                text += line.text.capacity();
                highlights += line.highlights.as_ref().map_or(0, |ranges| {
                    ranges.capacity() * size_of::<std::ops::Range<usize>>()
                });
            }
        }
    }
    println!(
        "source_text_capacity_bytes={text} line_vector_capacity_bytes={lines} highlight_capacity_bytes={highlights}"
    );
    drop(old);
    drop(new);
    report("prepared_inputs_dropped", None);
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    let widget = Diff::new(&document)
        .mode(ViewMode::Split)
        .wrap(true)
        .context_lines(Some(3));
    let mut state = DiffState::new();
    let file = document.file_fold(0).unwrap();
    assert!(state.set_file_expanded(&file, false));
    (&widget).render(area, &mut buffer, &mut state);
    report("initial_file_closed", Some(&state));
    assert!(state.set_file_expanded(&file, true));
    (&widget).render(area, &mut buffer, &mut state);
    report("file_open_context_closed", Some(&state));
    let folds = state.context_folds().to_vec();
    for fold in &folds {
        assert!(state.set_context_expanded(fold, true));
    }
    (&widget).render(area, &mut buffer, &mut state);
    report("context_open", Some(&state));
    state.set_search(&document, "hello", None);
    report("search_all_lines", Some(&state));
    for _ in 0..1_000 {
        state.scroll_lines(1);
        (&widget).render(area, &mut buffer, &mut state);
    }
    report("after_1000_frames", Some(&state));
    std::hint::black_box((&document, &state, &buffer));
}

fn report(phase: &str, state: Option<&DiffState>) {
    // ps reports resident KiB on macOS and Linux. Unsupported systems remain explicit.
    let rss = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unavailable".into());
    println!(
        "phase={phase} rss_kib={} rows={}",
        rss.trim(),
        state.map_or(0, DiffState::row_count)
    );
}
