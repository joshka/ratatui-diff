//! Optional syntax preparation and viewport costs, excluding source comparison and asset cold
//! start.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffState, DiffTheme, FileSyntax, SyntaxHighlighter, SyntaxSource, ViewMode,
};

pub(super) fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax");
    group.sample_size(10);
    let highlighter = SyntaxHighlighter::bundled("Coldark-Dark").unwrap();
    group.bench_function("contrast-theme", |b| {
        b.iter(|| {
            SyntaxHighlighter::bundled("Coldark-Dark")
                .unwrap()
                .contrast_with(DiffTheme::aardvark_ink())
                .unwrap()
        })
    });
    for count in [1_000, 100_000] {
        let text: String = (0..count)
            .map(|n| format!("let value_{n} = \"界 café\"; // comment\n"))
            .collect();
        let document = DiffDocument::from_text("", &text);
        let inputs = [FileSyntax {
            file: 0,
            language: "rs",
            source: SyntaxSource::Full {
                old: "",
                new: &text,
            },
        }];
        group.bench_function(BenchmarkId::new("prepare", count), |b| {
            b.iter(|| {
                highlighter
                    .prepare(black_box(&document), black_box(&inputs))
                    .unwrap()
            })
        });
        let styles = highlighter.prepare(&document, &inputs).unwrap();
        for mode in [ViewMode::Unified, ViewMode::Split] {
            for enabled in [false, true] {
                let widget = Diff::new(&document).mode(mode).wrap(true);
                let widget = if enabled {
                    widget.syntax_styles(&styles).unwrap()
                } else {
                    widget
                };
                let label = format!("{mode:?}-enabled-{enabled}");
                let area = Rect::new(0, 0, 100, 30);
                let mut buffer = Buffer::empty(area);
                group.bench_function(BenchmarkId::new(format!("layout-{label}"), count), |b| {
                    b.iter_batched(
                        DiffState::new,
                        |mut state| {
                            (&widget).render(area, &mut buffer, &mut state);
                            black_box(state)
                        },
                        BatchSize::LargeInput,
                    )
                });
                let mut state = DiffState::new();
                (&widget).render(area, &mut buffer, &mut state);
                group.bench_function(BenchmarkId::new(format!("scroll-{label}"), count), |b| {
                    b.iter(|| {
                        state.scroll_lines(1);
                        if state.offset() + 30 >= state.row_count() {
                            state.start();
                        }
                        (&widget).render(area, &mut buffer, &mut state);
                        black_box(&buffer);
                    })
                });
            }
        }
    }
    group.finish();
}
