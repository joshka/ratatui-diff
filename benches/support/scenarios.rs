use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, ViewMode};

pub fn context_sources(count: usize) -> (String, String) {
    let old = (0..count)
        .map(|n| format!("line {n}: 界 e\u{301} 👩‍💻 hello\n"))
        .collect::<String>();
    let new = old.replacen("line 500:", "changed 500:", 1);
    (old, new)
}

pub fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenarios");
    group.sample_size(10);
    for count in [1_000, 100_000] {
        let (old, new) = context_sources(count);
        group.bench_function(BenchmarkId::new("compare-retained", count), |b| {
            b.iter(|| DiffDocument::compare(black_box(&old), black_box(&new), usize::MAX))
        });
        let document = DiffDocument::compare(&old, &new, usize::MAX);
        group.bench_function(BenchmarkId::new("validate-structured", count), |b| {
            b.iter_batched(
                || document.files().to_vec(),
                |files| black_box(DiffDocument::new(files).unwrap()),
                BatchSize::LargeInput,
            );
        });
        for mode in [ViewMode::Unified, ViewMode::Split] {
            for wrap in [false, true] {
                let widget = Diff::new(&document)
                    .mode(mode)
                    .wrap(wrap)
                    .context_lines(Some(3));
                let area = Rect::new(0, 0, 100, 30);
                let mut buffer = Buffer::empty(area);
                let label = format!("{mode:?}-wrap-{wrap}");
                for closed in [false, true] {
                    let label = format!("{label}-file-closed-{closed}");
                    let file = document.file_fold(0).unwrap();
                    group.bench_function(
                        BenchmarkId::new(format!("initial-{label}"), count),
                        |b| {
                            b.iter_batched(
                                || {
                                    let mut state = DiffState::new();
                                    assert!(state.set_file_expanded(&file, !closed));
                                    state
                                },
                                |mut state| {
                                    (&widget).render(area, &mut buffer, &mut state);
                                    black_box(state)
                                },
                                BatchSize::LargeInput,
                            );
                        },
                    );
                    let mut state = DiffState::new();
                    assert!(state.set_file_expanded(&file, !closed));
                    (&widget).render(area, &mut buffer, &mut state);
                    assert!(!state.context_folds().is_empty());
                    group.bench_function(BenchmarkId::new(format!("steady-{label}"), count), |b| {
                        b.iter(|| {
                            state.scroll_lines(1);
                            if state.offset() + 30 >= state.row_count() {
                                state.start();
                            }
                            (&widget).render(area, &mut buffer, &mut state);
                            black_box(&buffer);
                        });
                    });
                    let first = state.context_folds()[0].clone();
                    let largest = state
                        .context_folds()
                        .iter()
                        .max_by_key(|fold| fold.line_count())
                        .unwrap()
                        .clone();
                    for (name, fold) in [
                        ("context-rebuild", first),
                        ("largest-context-rebuild", largest),
                    ] {
                        let mut expanded = state.context_expanded(&fold);
                        group.bench_function(
                            BenchmarkId::new(format!("{name}-{label}"), count),
                            |b| {
                                b.iter(|| {
                                    expanded = !expanded;
                                    assert!(state.set_context_expanded(&fold, expanded));
                                    (&widget).render(area, &mut buffer, &mut state);
                                    black_box(&state);
                                });
                            },
                        );
                        assert!(state.set_context_expanded(&fold, false));
                        (&widget).render(area, &mut buffer, &mut state);
                    }
                }
                let mut state = DiffState::new();
                (&widget).render(area, &mut buffer, &mut state);
                let folds = state.context_folds().to_vec();
                for fold in &folds {
                    assert!(state.set_context_expanded(fold, true));
                }
                (&widget).render(area, &mut buffer, &mut state);
                group.bench_function(
                    BenchmarkId::new(format!("context-open-steady-{label}"), count),
                    |b| {
                        b.iter(|| {
                            state.scroll_lines(1);
                            if state.offset() + 30 >= state.row_count() {
                                state.start();
                            }
                            (&widget).render(area, &mut buffer, &mut state);
                            black_box(&buffer);
                        });
                    },
                );
                let mut width = 100;
                group.bench_function(
                    BenchmarkId::new(format!("context-open-resize-{label}"), count),
                    |b| {
                        b.iter(|| {
                            width = if width == 100 { 99 } else { 100 };
                            (&widget).render(Rect::new(0, 0, width, 30), &mut buffer, &mut state);
                            black_box(&state);
                        });
                    },
                );
                let file = document.file_fold(0).unwrap();
                let mut expanded = true;
                group.bench_function(
                    BenchmarkId::new(format!("file-rebuild-{label}"), count),
                    |b| {
                        b.iter(|| {
                            expanded = !expanded;
                            assert!(state.set_file_expanded(&file, expanded));
                            (&widget).render(area, &mut buffer, &mut state);
                            black_box(&state);
                        });
                    },
                );
                group.bench_function(
                    BenchmarkId::new(format!("search-update-{label}"), count),
                    |b| {
                        b.iter(|| {
                            state.set_search(&document, "hello", None);
                            black_box(&state);
                            state.clear_search();
                        });
                    },
                );
            }
        }
    }
    for (name, old, new) in [
        (
            "long-ascii",
            String::new(),
            format!("{}\n", "a".repeat(100_000)),
        ),
        (
            "long-unicode",
            String::new(),
            format!("{}\n", "界e\u{301}👩‍💻".repeat(10_000)),
        ),
        ("uneven", "old line\n".repeat(128), "new line\n".repeat(257)),
        ("repeated", "a\nb\n".repeat(2_000), "b\na\n".repeat(2_000)),
    ] {
        group.bench_function(format!("compare-{name}"), |b| {
            b.iter(|| DiffDocument::from_text(black_box(&old), black_box(&new)))
        });
        let document = DiffDocument::from_text(&old, &new);
        for wrap in [false, true] {
            let widget = Diff::new(&document).mode(ViewMode::Split).wrap(wrap);
            let area = Rect::new(0, 0, 100, 30);
            let mut buffer = Buffer::empty(area);
            group.bench_function(format!("initial-{name}-wrap-{wrap}"), |b| {
                b.iter_batched(
                    DiffState::new,
                    |mut state| {
                        (&widget).render(area, &mut buffer, &mut state);
                        black_box(state)
                    },
                    BatchSize::LargeInput,
                );
            });
            let mut state = DiffState::new();
            (&widget).render(area, &mut buffer, &mut state);
            let mut width = 100;
            group.bench_function(format!("resize-{name}-wrap-{wrap}"), |b| {
                b.iter(|| {
                    width = if width == 100 { 99 } else { 100 };
                    (&widget).render(Rect::new(0, 0, width, 30), &mut buffer, &mut state);
                    black_box(&state);
                });
            });
            group.bench_function(format!("steady-{name}-wrap-{wrap}"), |b| {
                b.iter(|| {
                    state.scroll_lines(1);
                    if state.offset() + 30 >= state.row_count() {
                        state.start();
                    }
                    (&widget).render(area, &mut buffer, &mut state);
                    black_box(&buffer);
                });
            });
        }
    }
    group.finish();
}
