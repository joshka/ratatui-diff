//! Maintainer folding/resize profiling driver; not a host integration example.
//! See `docs/folding-resize-performance.md` for phases and measurement limitations.
use std::hint::black_box;
use std::time::{Duration, Instant};

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, Side, SourcePosition, ViewMode};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let files: usize = args.get(1).map_or(1, |s| s.parse().unwrap());
    let lines: usize = args.get(2).map_or(1_000, |s| s.parse().unwrap());
    let phase = args.get(3).map_or("resize", String::as_str);
    let mode = if args.get(4).is_some_and(|s| s == "split") {
        ViewMode::Split
    } else {
        ViewMode::Unified
    };
    let wrap = args.get(5).is_some_and(|s| s == "wrap");
    let closed = args.get(6).is_some_and(|s| s == "closed");
    let iterations: usize = args.get(7).map_or(20, |s| s.parse().unwrap());
    assert!(files > 0 && lines >= 100 && iterations > 0);
    assert!(matches!(
        phase,
        "initial"
            | "context-state"
            | "file-state"
            | "context-first"
            | "file-first"
            | "resize"
            | "resize-open"
            | "steady"
            | "steady-open"
            | "navigation"
    ));
    let check_only = std::env::var_os("RATATUI_DIFF_PROFILE_CHECK").is_some();
    let old: String = (0..lines)
        .map(|n| format!("line {n:06}: 界 e\u{301} 👩‍💻 retained context and wrapping payload\n"))
        .collect();
    let new: String = old
        .lines()
        .enumerate()
        .map(|(n, line)| {
            if n % 100 == 50 {
                format!("changed {line}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    let compared = DiffDocument::compare(&old, &new, usize::MAX);
    let mut entries = Vec::with_capacity(files);
    for n in 0..files {
        let mut file = compared.files()[0].clone();
        file.old_path = Some(format!("file-{n}.rs"));
        file.new_path = file.old_path.clone();
        entries.push(file);
    }
    let document = DiffDocument::new(entries).unwrap();
    let widget = Diff::new(&document)
        .mode(mode)
        .wrap(wrap)
        .context_lines(Some(3));
    let area = Rect::new(0, 0, 100, 30);
    let mut buffer = Buffer::empty(area);
    let fresh = || {
        let mut state = DiffState::new();
        for n in 0..files {
            assert!(state.set_file_expanded(&document.file_fold(n).unwrap(), !closed));
        }
        state
    };
    let mut state = fresh();
    (&widget).render(area, &mut buffer, &mut state);
    let fold = state.context_folds()[0].clone();
    let file = document.file_fold(0).unwrap();
    if phase.contains("open") {
        for fold in state.context_folds().to_vec() {
            assert!(state.set_context_expanded(&fold, true));
        }
        (&widget).render(area, &mut buffer, &mut state);
    }
    let mut elapsed = Duration::ZERO;
    for n in 0..iterations {
        if phase == "initial" {
            state = fresh();
        }
        if phase == "context-first" {
            assert!(state.set_context_expanded(&fold, n % 2 == 0));
        }
        if phase == "file-first" {
            assert!(state.set_file_expanded(&file, n % 2 != 0));
        }
        let start = Instant::now();
        match phase {
            "context-state" => {
                black_box(state.set_context_expanded(&fold, n % 2 == 0));
            }
            "file-state" => {
                black_box(state.set_file_expanded(&file, n % 2 != 0));
            }
            "navigation" => {
                black_box(state.scroll_to_source(SourcePosition {
                    file: files - 1,
                    side: Side::New,
                    line: lines,
                }));
            }
            _ => {
                let width = if phase.starts_with("resize") && n % 2 == 0 {
                    40
                } else {
                    100
                };
                (&widget).render(Rect::new(0, 0, width, 30), &mut buffer, &mut state);
                black_box(&buffer);
            }
        }
        elapsed += start.elapsed();
    }
    if check_only {
        assert_eq!(state.file_folds().len(), files);
        assert!(!state.context_folds().is_empty());
        println!("check passed: {phase}, {files} files, {lines} lines/file");
        return;
    }
    println!(
        "{files},{lines},{phase},{mode:?},{wrap},{closed},{iterations},{:.3},{}",
        elapsed.as_secs_f64() * 1e6 / iterations as f64,
        state.context_folds().len()
    );
}
