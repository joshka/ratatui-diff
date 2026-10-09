//! Preparation, layout, resize, and scrolling baselines.
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, Side, SourceBoundary, SourcePosition, SourceSelection, ViewMode,
};

fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewer");
    group.sample_size(10);
    for count in [1_000, 100_000] {
        let text = (0..count)
            .map(|n| format!("line {n}: 界 hello world\n"))
            .collect::<String>();
        group.bench_with_input(BenchmarkId::new("prepare", count), &text, |b, t| {
            b.iter(|| DiffDocument::from_text("", black_box(t)))
        });
        let document = DiffDocument::from_text("", &text);
        for mode in [ViewMode::Unified, ViewMode::Split] {
            for wrap in [false, true] {
                let widget = Diff::new(&document).mode(mode).wrap(wrap);
                let area = Rect::new(0, 0, 100, 30);
                let mut buf = Buffer::empty(area);
                let label = format!("{mode:?}-wrap-{wrap}");
                group.bench_function(BenchmarkId::new(format!("layout-{label}"), count), |b| {
                    b.iter(|| {
                        let mut state = DiffState::new();
                        (&widget).render(area, &mut buf, &mut state);
                        black_box(state);
                    })
                });
                let mut state = DiffState::new();
                let mut state_width = 100;
                (&widget).render(area, &mut buf, &mut state);
                group.bench_function(BenchmarkId::new(format!("hit-{label}"), count), |b| {
                    let x = if mode == ViewMode::Split { 75 } else { 25 };
                    b.iter(|| black_box(state.hit_test(black_box(x), black_box(12))))
                });
                group.bench_function(BenchmarkId::new(format!("scroll-{label}"), count), |b| {
                    b.iter(|| {
                        state.scroll_lines(1);
                        if state.offset() + 30 >= state.row_count() {
                            state.start();
                        }
                        (&widget).render(area, &mut buf, &mut state);
                        black_box(&buf);
                    })
                });
                let anchor = SourceBoundary {
                    position: SourcePosition {
                        file: 0,
                        side: Side::New,
                        line: 1,
                    },
                    byte: 0,
                };
                let last = document.files()[0].hunks[0].lines.last().unwrap();
                let focus = SourceBoundary {
                    position: SourcePosition {
                        line: count,
                        ..anchor.position
                    },
                    byte: last.text.len() + 1,
                };
                state.set_selection(&document, SourceSelection { anchor, focus });
                group.bench_function(
                    BenchmarkId::new(format!("selection-scroll-{label}"), count),
                    |b| {
                        b.iter(|| {
                            state.scroll_lines(1);
                            if state.offset() + 30 >= state.row_count() {
                                state.start();
                            }
                            (&widget).render(area, &mut buf, &mut state);
                            black_box(&buf);
                        })
                    },
                );
                state.clear_selection();
                group.bench_function(BenchmarkId::new(format!("resize-{label}"), count), |b| {
                    b.iter(|| {
                        let next = if state_width == 100 { 99 } else { 100 };
                        state_width = next;
                        (&widget).render(Rect::new(0, 0, next, 30), &mut buf, &mut state);
                        black_box(&state);
                    })
                });
            }
        }
    }
    let old = (0..200)
        .map(|n| format!("old {n}: {}\n", "a".repeat(4096)))
        .collect::<String>();
    let new = (0..200)
        .map(|n| format!("new {n}: {}\n", "b".repeat(4096)))
        .collect::<String>();
    group.bench_function("dissimilar-long-lines", |b| {
        b.iter(|| DiffDocument::from_text(black_box(&old), black_box(&new)))
    });
    group.finish();
}
criterion_group!(viewer, benches);
criterion_main!(viewer);
