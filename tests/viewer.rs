//! Public parsing, presentation, and navigation contracts.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffFile, DiffLine, DiffState, DiffTheme, Hunk, LineKind, Side,
    SourcePosition, ViewMode,
};

fn render(widget: &Diff<'_>, state: &mut DiffState, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer, state);
    buffer
}
fn text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn parses_multiple_files_and_numbered_lines() {
    let patch = "--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@\n same\n-old\n+new\n--- a/b\n+++ b/b\n@@ -0,0 +1 @@\n+created\n";
    let document = DiffDocument::parse(patch).unwrap();
    assert_eq!(document.files().len(), 2);
    assert_eq!(document.files()[0].hunks[0].lines[2].new, Some(2));
    assert_eq!(document.files()[1].hunks[0].lines[0].old, None);
}
#[test]
fn crlf_headers_do_not_change_payload_endings() {
    let patch = "diff --git a/a b/a\r\nindex 1234567..7654321 100644\r\n--- a/a\r\n+++ b/a\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n";
    let d = DiffDocument::parse(patch).unwrap();
    assert_eq!(d.files()[0].hunks[0].lines[0].text, "old\r");
}
#[test]
fn keeps_final_newline_markers() {
    let d = DiffDocument::parse(
        "--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n",
    )
    .unwrap();
    assert!(!d.files()[0].hunks[0].lines[0].terminated);
    assert!(d.files()[0].hunks[0].lines[1].terminated);
}
#[test]
fn rejects_truncated_hunks_and_combined_diffs() {
    assert!(DiffDocument::parse("--- a/a\n+++ b/a\n@@ -1,2 +1 @@\n-old\n+new\n").is_err());
    let e = DiffDocument::parse("diff --cc a\n").unwrap_err();
    assert_eq!(e.offset, Some(0));
}
#[test]
fn empty_patch_is_empty() {
    assert!(DiffDocument::parse("").unwrap().files().is_empty());
}
#[test]
fn quoted_paths_and_metadata_only_rename() {
    let patch = "diff --git \"a/old name\" \"b/new name\"\nsimilarity index 100%\nrename from old name\nrename to new name\n";
    let d = DiffDocument::parse(patch).unwrap();
    assert!(
        d.files()[0]
            .old_path
            .as_deref()
            .unwrap()
            .contains("old name")
    );
    assert!(
        d.files()[0]
            .metadata
            .iter()
            .any(|m| m.starts_with("rename to"))
    );
}
#[test]
fn binary_summary_is_opaque() {
    let d = DiffDocument::parse(
        "diff --git a/a b/a\nindex 1234567..7654321 100644\nBinary files a/a and b/a differ\n",
    )
    .unwrap();
    assert!(d.files()[0].binary);
    assert!(d.files()[0].hunks.is_empty());
}
#[test]
fn validates_ranges_and_preserves_supplied_highlights() {
    let line = DiffLine {
        highlights: Some(std::iter::once(0..3).collect()),
        ..DiffLine::new(LineKind::Delete, Some(1), None, "old\n")
    };
    let file = DiffFile {
        old_path: Some("a".into()),
        new_path: Some("a".into()),
        metadata: vec![],
        binary: false,
        hunks: vec![Hunk {
            old: 1..2,
            new: 1..2,
            lines: vec![
                line,
                DiffLine::new(LineKind::Insert, None, Some(1), "new\n"),
            ],
        }],
    };
    let d = DiffDocument::new(vec![file.clone()]).unwrap();
    assert_eq!(
        d.files()[0].hunks[0].lines[0].highlights,
        Some(std::iter::once(0..3).collect())
    );
    let mut broken = file;
    broken.hunks[0].old = 1..3;
    assert!(DiffDocument::new(vec![broken]).is_err());
}
#[test]
fn validates_utf8_highlight_boundaries() {
    let line = DiffLine {
        highlights: Some(std::iter::once(1..2).collect()),
        ..DiffLine::new(LineKind::Insert, None, Some(1), "界\n")
    };
    let file = DiffFile {
        old_path: None,
        new_path: Some("a".into()),
        metadata: vec![],
        binary: false,
        hunks: vec![Hunk {
            old: 0..0,
            new: 1..2,
            lines: vec![line],
        }],
    };
    assert!(DiffDocument::new(vec![file]).is_err());
}
#[test]
fn draws_both_modes_and_inline_styles() {
    let d = DiffDocument::from_text("hello old\n", "hello new\n");
    for mode in [ViewMode::Unified, ViewMode::Split] {
        let mut state = DiffState::new();
        let b = render(
            &Diff::new(&d).mode(mode).theme(DiffTheme::monochrome()),
            &mut state,
            60,
            8,
        );
        let output = text(&b);
        assert!(output.contains("hello old"));
        assert!(output.contains("hello new"));
        assert!(
            b.content
                .iter()
                .any(|c| c.modifier.contains(Modifier::UNDERLINED))
        );
    }
}
#[test]
fn split_alignment_pads_uneven_changes() {
    let d = DiffDocument::from_text("a\nb\n", "c\n");
    let mut state = DiffState::new();
    let b = render(&Diff::new(&d).mode(ViewMode::Split), &mut state, 30, 6);
    assert_eq!(state.row_count(), 4);
    assert!(text(&b).contains("2 -b"));
}
#[test]
fn controls_are_visible_and_tabs_follow_stops() {
    let d = DiffDocument::from_text("", "a\tb\x1b[31m\n");
    let mut state = DiffState::new();
    let b = render(
        &Diff::new(&d).line_numbers(false).whitespace(true),
        &mut state,
        60,
        5,
    );
    let output = text(&b);
    assert!(output.contains("a→  b\\u{1b}[31m"));
    assert!(!output.contains('\x1b'));
}
#[test]
fn wide_graphemes_are_not_split_at_horizontal_edges() {
    let d = DiffDocument::from_text("", "界a界b\n");
    let mut state = DiffState::new();
    render(&Diff::new(&d).line_numbers(false), &mut state, 6, 3);
    state.scroll_horizontal(1);
    let b = render(&Diff::new(&d).line_numbers(false), &mut state, 6, 3);
    assert_eq!(b[(1, 2)].symbol(), " ");
    assert_eq!(b[(2, 2)].symbol(), "a");
}
#[test]
fn wrapped_rows_keep_source_mapping_and_resize_anchor() {
    let d = DiffDocument::from_text("", "a long line with words and 界\nsecond\nthird\n");
    let mut state = DiffState::new();
    render(&Diff::new(&d).wrap(true), &mut state, 15, 3);
    assert!(state.row_count() > 5);
    assert!(state.scroll_to_source(SourcePosition {
        file: 0,
        side: Side::New,
        line: 2
    }));
    let expected = state.source_at(state.offset(), Side::New);
    render(
        &Diff::new(&d).mode(ViewMode::Split).wrap(true),
        &mut state,
        30,
        3,
    );
    assert_eq!(state.source_at(state.offset(), Side::New), expected);
    assert!(!state.scroll_to_source(SourcePosition {
        file: 0,
        side: Side::Old,
        line: 100
    }));
}
#[test]
fn line_page_and_half_page_navigation_clamps() {
    let new = (0..100).map(|n| format!("{n}\n")).collect::<String>();
    let d = DiffDocument::from_text("", &new);
    let mut state = DiffState::new();
    render(&Diff::new(&d), &mut state, 30, 10);
    state.scroll_pages(1);
    assert_eq!(state.offset(), 10);
    state.scroll_half_pages(-1);
    assert_eq!(state.offset(), 5);
    state.end();
    assert_eq!(state.offset(), state.row_count() - 10);
    state.scroll_lines(isize::MAX);
    assert_eq!(state.offset(), state.row_count() - 10);
    state.start();
    state.scroll_lines(-1);
    assert_eq!(state.offset(), 0);
}
#[test]
fn hunk_and_file_navigation_uses_headers() {
    let d=DiffDocument::parse("diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n-c\n+d\n").unwrap();
    let mut s = DiffState::new();
    render(&Diff::new(&d), &mut s, 50, 1);
    s.next_hunk();
    let first = s.offset();
    assert!(first > 0);
    s.next_file();
    assert!(s.offset() > first);
    s.previous_file();
    assert_eq!(s.offset(), 0);
    s.next_hunk();
    s.next_hunk();
    s.previous_hunk();
    assert_eq!(s.offset(), first);
}
#[test]
fn tiny_areas_and_document_replacement_are_safe() {
    let d = DiffDocument::from_text("a\n", "b\n");
    let empty = DiffDocument::parse("").unwrap();
    let mut s = DiffState::new();
    for width in 0..10 {
        for height in 0..3 {
            render(
                &Diff::new(&d).mode(ViewMode::Split).wrap(true).tab_width(0),
                &mut s,
                width,
                height,
            );
        }
    }
    s.end();
    render(&Diff::new(&empty), &mut s, 10, 2);
    assert_eq!(s.offset(), 0);
    assert_eq!(s.row_count(), 0);
}
#[test]
fn bounded_refinement_falls_back_and_theme_change_preserves_viewport() {
    let old = format!("{}\n", "a".repeat(9000));
    let new = format!("{}\n", "b".repeat(9000));
    let d = DiffDocument::from_text(&old, &new);
    assert_eq!(d.files()[0].hunks[0].lines[0].highlights, None);
    let mut s = DiffState::new();
    render(&Diff::new(&d), &mut s, 20, 1);
    s.scroll_lines(2);
    render(&Diff::new(&d).theme(DiffTheme::light()), &mut s, 20, 1);
    assert_eq!(s.offset(), 2);
}

#[test]
fn rejects_overflowing_ranges_without_panicking() {
    let patch = format!(
        "--- a/a\n+++ b/a\n@@ -{},1 +1,1 @@\n-old\n+new\n",
        usize::MAX
    );
    assert!(DiffDocument::parse(&patch).is_err());
}

#[test]
fn structured_hunks_must_not_overlap() {
    let line = DiffLine::new(LineKind::Context, Some(1), Some(1), "same\n");
    let hunk = Hunk {
        old: 1..2,
        new: 1..2,
        lines: vec![line],
    };
    let file = DiffFile {
        old_path: Some("a".into()),
        new_path: Some("a".into()),
        metadata: vec![],
        binary: false,
        hunks: vec![hunk.clone(), hunk],
    };
    assert!(DiffDocument::new(vec![file]).is_err());
}

#[test]
fn combining_clusters_survive_wrapping_and_styles_reset() {
    let d = DiffDocument::from_text("e\u{301} old\n", "e\u{301} new\n");
    let area = Rect::new(0, 0, 12, 12);
    let mut buffer = Buffer::empty(area);
    let mut state = DiffState::new();
    (&Diff::new(&d).wrap(true).theme(DiffTheme::monochrome())).render(
        area,
        &mut buffer,
        &mut state,
    );
    assert!(text(&buffer).contains("e\u{301}"));
    assert!(
        buffer
            .content
            .iter()
            .any(|c| c.modifier.contains(Modifier::UNDERLINED))
    );
    let mut plain = DiffTheme::dark();
    plain.insert_word = plain.insert;
    plain.delete_word = plain.delete;
    (&Diff::new(&d).wrap(true).theme(plain)).render(area, &mut buffer, &mut state);
    assert!(
        !buffer
            .content
            .iter()
            .any(|c| c.modifier.contains(Modifier::UNDERLINED))
    );
}

#[test]
fn hunk_start_zero_does_not_mean_source_line_zero() {
    let d = DiffDocument::parse("--- /dev/null\n+++ b/new\n@@ -0,0 +1,1 @@\n+created\n").unwrap();
    assert_eq!(d.files()[0].old_path, None);
    assert_eq!(d.files()[0].hunks[0].lines[0].new, Some(1));
}

#[test]
fn generated_empty_ranges_follow_unified_header_conventions() {
    let added = DiffDocument::from_text("", "a\n");
    assert_eq!(added.files()[0].hunks[0].old, 0..0);
    let deleted = DiffDocument::from_text("a\n", "");
    assert_eq!(deleted.files()[0].hunks[0].new, 0..0);
}

#[test]
fn inline_emphasis_can_be_disabled() {
    let d = DiffDocument::from_text("old\n", "new\n");
    let mut state = DiffState::new();
    let buffer = render(
        &Diff::new(&d)
            .theme(DiffTheme::monochrome())
            .word_highlights(false),
        &mut state,
        20,
        6,
    );
    assert!(
        !buffer
            .content
            .iter()
            .any(|c| c.modifier.contains(Modifier::UNDERLINED))
    );
}

#[test]
fn structured_documents_reject_impossible_file_and_line_shapes() {
    let mut file = DiffFile {
        old_path: None,
        new_path: None,
        metadata: vec![],
        binary: false,
        hunks: vec![],
    };
    assert!(DiffDocument::new(vec![file.clone()]).is_err());
    file.new_path = Some("new".into());
    file.hunks = vec![Hunk {
        old: 0..0,
        new: 1..3,
        lines: vec![
            DiffLine::new(LineKind::Insert, None, Some(1), "first"),
            DiffLine::new(LineKind::Insert, None, Some(2), "second\n"),
        ],
    }];
    assert!(DiffDocument::new(vec![file]).is_err());
}

#[test]
fn automatic_highlights_join_phrases_but_preserve_unchanged_boundaries() {
    for (old, new, left, right) in [
        (
            "  red fox keep blue jay  \n",
            "  green owl keep white swan  \n",
            vec![2..9, 15..23],
            vec![2..11, 17..27],
        ),
        (
            " α\tβ same \n",
            " γ\tδ same \n",
            std::iter::once(1..6).collect(),
            std::iter::once(1..6).collect(),
        ),
    ] {
        let document = DiffDocument::from_text(old, new);
        let lines = &document.files()[0].hunks[0].lines;
        assert_eq!(lines[0].highlights.as_ref(), Some(&left));
        assert_eq!(lines[1].highlights.as_ref(), Some(&right));

        let mut files = document.files().to_vec();
        let explicit = vec![2..5, 6..9];
        // Deliberately exclude the space between two caller-selected words.
        if old.is_ascii() {
            files[0].hunks[0].lines[0].highlights = Some(explicit.clone());
            let supplied = DiffDocument::new(files).unwrap();
            assert_eq!(
                supplied.files()[0].hunks[0].lines[0].highlights,
                Some(explicit)
            );
        }
    }
}

#[test]
fn only_generated_whitespace_markers_are_dimmed() {
    let document = DiffDocument::from_text("", "a · →\twords here\n");
    let mut state = DiffState::new();
    let diff = Diff::new(&document).line_numbers(false).whitespace(true);
    let buffer = render(&diff, &mut state, 40, 4);
    // The marker takes one cell. Literal dot/arrow remain source text.
    for (x, symbol, dim) in [
        (2, "·", true),
        (3, "·", false),
        (5, "→", false),
        (6, "→", true),
    ] {
        assert_eq!(buffer[(x, 2)].symbol(), symbol);
        assert_eq!(buffer[(x, 2)].modifier.contains(Modifier::DIM), dim);
    }
    let hidden = render(&diff.whitespace(false), &mut state, 40, 4);
    assert!(
        hidden
            .content
            .iter()
            .all(|cell| !cell.modifier.contains(Modifier::DIM))
    );
}

#[test]
fn compact_gutters_keep_markers_adjacent_to_source() {
    let document = DiffDocument::from_text("a\n", "b\n");
    for (mode, numbers, expected) in [
        (ViewMode::Unified, true, "1   -a"),
        (ViewMode::Unified, false, "-a"),
        (ViewMode::Split, true, "1 -a"),
        (ViewMode::Split, false, "-a"),
    ] {
        let mut state = DiffState::new();
        let diff = Diff::new(&document).mode(mode).line_numbers(numbers);
        let buffer = render(&diff, &mut state, 25, 4);
        assert!(text(&buffer).contains(expected), "{}", text(&buffer));
    }
}

#[test]
fn wrapped_gutters_distinguish_continuations_from_empty_alignment_cells() {
    let document = DiffDocument::from_text("abcdefghij\n\n", "klmnopqrst\nuvwxyzabcd\n");
    let mut state = DiffState::new();
    let diff = Diff::new(&document).mode(ViewMode::Split).wrap(true);
    let buffer = render(&diff, &mut state, 19, 8);
    assert_eq!(buffer[(0, 2)].symbol(), "1");
    assert_eq!(buffer[(0, 3)].symbol(), "↪");
    assert!(!buffer[(0, 2)].modifier.contains(Modifier::DIM));
    assert!(buffer[(0, 3)].modifier.contains(Modifier::DIM));
    assert_eq!(buffer[(0, 4)].symbol(), "2");
    // A genuine blank source line has a number; its alignment padding does not.
    for x in 0..9 {
        assert_eq!(buffer[(x, 5)].symbol(), " ");
    }
    assert_eq!(buffer[(10, 5)].symbol(), "↪");
    assert_eq!(state.source_at(3, Side::Old).unwrap().line, 1);
    assert_eq!(state.source_at(5, Side::New).unwrap().line, 2);

    render(&diff, &mut state, 19, 2);
    state.scroll_lines(3);
    let scrolled = render(&diff, &mut state, 19, 2);
    assert_eq!(scrolled[(0, 0)].symbol(), "↪");
    assert_eq!(state.source_at(state.offset(), Side::Old).unwrap().line, 1);
}

#[test]
fn unified_context_has_one_continuation_cue_for_both_source_numbers() {
    let document = DiffDocument::from_text("context-long\nold\n", "context-long\nnew\n");
    let mut state = DiffState::new();
    let buffer = render(&Diff::new(&document).wrap(true), &mut state, 16, 8);
    assert_eq!(buffer[(0, 2)].symbol(), "1");
    assert_eq!(buffer[(2, 2)].symbol(), "1");
    assert_eq!(buffer[(0, 3)].symbol(), " ");
    assert_eq!(buffer[(2, 3)].symbol(), "↪");
    assert!(buffer[(2, 3)].modifier.contains(Modifier::DIM));
    assert_eq!(state.source_at(3, Side::Old).unwrap().line, 1);
    assert_eq!(state.source_at(3, Side::New).unwrap().line, 1);
}
