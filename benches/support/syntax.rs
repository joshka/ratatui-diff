//! Optional syntax preparation and viewport costs, excluding source comparison and asset cold
//! start.

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

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
    group.bench_function("warm-assets-theme", |b| {
        b.iter(|| SyntaxHighlighter::bundled(black_box("Coldark-Dark")).unwrap())
    });
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
    for name in ["rust", "multiline", "unicode", "long-line"] {
        let text = fixture(name, 1_000);
        let document = DiffDocument::from_text("", &text);
        for (label, source) in [
            (
                "full",
                SyntaxSource::Full {
                    old: "",
                    new: &text,
                },
            ),
            ("retained", SyntaxSource::Retained),
        ] {
            let inputs = [FileSyntax {
                file: 0,
                language: "rs",
                source,
            }];
            group.bench_function(format!("workload-{name}-{label}"), |b| {
                b.iter(|| {
                    highlighter
                        .prepare(black_box(&document), black_box(&inputs))
                        .unwrap()
                })
            });
        }
    }
    // Both snapshots have a long identical prefix and one inserted line at EOF. Only three
    // context lines and the insertion are retained; Full still parses both prefixes.
    let old = fixture("multiline", 10_000);
    let new = format!("{old}let changed = 42;\n");
    let document = DiffDocument::from_text(&old, &new);
    for (label, source) in [
        (
            "full",
            SyntaxSource::Full {
                old: &old,
                new: &new,
            },
        ),
        ("retained", SyntaxSource::Retained),
    ] {
        let inputs = [FileSyntax {
            file: 0,
            language: "rs",
            source,
        }];
        group.bench_function(format!("sparse-eof-{label}"), |b| {
            b.iter(|| {
                highlighter
                    .prepare(black_box(&document), black_box(&inputs))
                    .unwrap()
            })
        });
    }
    group.finish();
}

fn fixture(name: &str, count: usize) -> String {
    (0..count)
        .map(|n| match name {
            "rust" => format!("fn value_{n}() -> usize {{ let value = {n}; value + 1 }}\n"),
            "multiline" => match n % 4 {
                0 => "/* opening comment\n".to_owned(),
                1 => "nested /* comment */ body\n".to_owned(),
                2 => "closing comment */\n".to_owned(),
                _ => format!("let value_{n} = r#\"raw string\"#;\n"),
            },
            "unicode" => format!("let value_{n} = \"界 cafe\u{301} 👩‍💻\"; // café\n"),
            "long-line" => format!("let value_{n} = \"{}\";\n", "界 café ".repeat(400)),
            _ => unreachable!("known benchmark fixture"),
        })
        .collect()
}

/// Fresh-process measurements: run the bench executable with RATATUI_DIFF_SYNTAX_PROBE set.
/// ps calls are outside timed regions. RSS differences include allocator/decoder scratch retention;
/// they are not an allocation census of the private style representation.
pub(super) fn probe() {
    report("baseline");
    let start = Instant::now();
    let themes = SyntaxHighlighter::themes().count();
    println!(
        "cold_assets_us={} themes={themes}",
        start.elapsed().as_micros()
    );
    report("assets");
    let start = Instant::now();
    let highlighter = SyntaxHighlighter::bundled("Coldark-Dark").unwrap();
    println!("first_theme_us={}", start.elapsed().as_micros());
    report("assets_theme");
    let start = Instant::now();
    let second = SyntaxHighlighter::bundled("Coldark-Dark").unwrap();
    println!("warm_assets_theme_us={}", start.elapsed().as_micros());
    drop(second);
    report("warm_theme_dropped");
    let count =
        std::env::var("RATATUI_DIFF_SYNTAX_LINES").map_or(100_000, |value| value.parse().unwrap());
    let text = fixture("unicode", count);
    let document = DiffDocument::from_text("", &text);
    println!("source_bytes={} lines={count}", text.len());
    report("document_inputs");
    let inputs = [FileSyntax {
        file: 0,
        language: "rs",
        source: SyntaxSource::Full {
            old: "",
            new: &text,
        },
    }];
    let start = Instant::now();
    let styles = highlighter.prepare(&document, &inputs).unwrap();
    println!("prepare_us={}", start.elapsed().as_micros());
    report("prepared_styles");
    drop(text);
    drop(highlighter);
    report("inputs_highlighter_dropped");
    let area = Rect::new(0, 0, 100, 30);
    let widget = Diff::new(&document).mode(ViewMode::Split).wrap(true);
    let mut buffer = Buffer::empty(area);
    let mut state = DiffState::new();
    (&widget).render(area, &mut buffer, &mut state);
    report("layout_without_styles");
    let widget = widget.syntax_styles(&styles).unwrap();
    for _ in 0..1_000 {
        state.scroll_lines(1);
        (&widget).render(area, &mut buffer, &mut state);
    }
    report("styled_1000_frames");
    black_box((&styles, &state, &buffer));
    drop(styles);
    report("styles_dropped");
}

fn report(phase: &str) {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps is required for the opt-in RSS probe");
    assert!(output.status.success());
    println!(
        "phase={phase} rss_kib={}",
        String::from_utf8(output.stdout).unwrap().trim()
    );
}
