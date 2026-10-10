//! Replacement pairing through public documents, layouts, and source contracts.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{
    Diff, DiffDocument, DiffFile, DiffLine, DiffState, HitTest, Hunk, LineKind, Side,
    SourceBoundary, SourcePosition, SourceSelection, ViewMode,
};

fn replacement(old: &[&str], new: &[&str]) -> DiffDocument {
    let mut lines = Vec::new();
    for (index, text) in old.iter().enumerate() {
        lines.push(DiffLine::new(LineKind::Delete, Some(index + 1), None, text));
    }
    for (index, text) in new.iter().enumerate() {
        lines.push(DiffLine::new(LineKind::Insert, None, Some(index + 1), text));
    }
    DiffDocument::new(vec![DiffFile {
        old_path: Some("old".into()),
        new_path: Some("new".into()),
        metadata: vec![],
        binary: false,
        hunks: vec![Hunk {
            old: 1..old.len() + 1,
            new: 1..new.len() + 1,
            lines,
        }],
    }])
    .unwrap()
}

fn draw(document: &DiffDocument, state: &mut DiffState, width: u16, wrap: bool) {
    let area = Rect::new(0, 0, width, 100);
    let widget = Diff::new(document).mode(ViewMode::Split).wrap(wrap);
    widget.render(area, &mut Buffer::empty(area), state);
}

fn pairs(document: &DiffDocument) -> Vec<(Option<usize>, Option<usize>)> {
    let mut state = DiffState::new();
    draw(document, &mut state, 100, false);
    (0..state.row_count())
        .filter_map(|row| {
            let old = state.source_at(row, Side::Old).map(|p| p.line);
            let new = state.source_at(row, Side::New).map(|p| p.line);
            (old.is_some() || new.is_some()).then_some((old, new))
        })
        .collect()
}

#[test]
fn inserted_unrelated_line_does_not_shift_setting_partners_or_highlights() {
    let old = "timeout_ms: 1000\nretries: 2\nendpoint: /api/v1\nlabel: café 界\n";
    let new = "# Audit requests before sending\ntimeout_ms: 2500\nretries: 4\nendpoint: /api/v2\nlabel: café 世界\n";
    let expected = vec![
        (None, Some(1)),
        (Some(1), Some(2)),
        (Some(2), Some(3)),
        (Some(3), Some(4)),
        (Some(4), Some(5)),
    ];
    let document = DiffDocument::from_text(old, new);
    assert_eq!(pairs(&document), expected);
    let lines = &document.files()[0].hunks[0].lines;
    let highlight: Vec<_> = std::iter::once(12..16).collect();
    assert_eq!(lines[0].highlights.as_ref().unwrap(), &highlight);
    assert_eq!(lines[4].highlights, None);
    assert_eq!(lines[5].highlights.as_ref().unwrap(), &highlight);
    let patch = "--- a/old\n+++ b/new\n@@ -1,4 +1,5 @@\n-timeout_ms: 1000\n-retries: 2\n-endpoint: /api/v1\n-label: café 界\n+# Audit requests before sending\n+timeout_ms: 2500\n+retries: 4\n+endpoint: /api/v2\n+label: café 世界\n";
    assert_eq!(pairs(&DiffDocument::parse(patch).unwrap()), expected);
}

#[test]
fn uneven_replacements_keep_anchors_and_source_order_inside_gaps() {
    let document = replacement(
        &["header: old\n", "removed extra\n", "tail: old\n"],
        &["header: new\n", "tail: new\n"],
    );
    assert_eq!(
        pairs(&document),
        vec![(Some(1), Some(1)), (Some(2), None), (Some(3), Some(2))]
    );
    let dissimilar = replacement(&["aaa\n", "bbb\n"], &["xxx\n", "yyy\n", "zzz\n"]);
    assert_eq!(
        pairs(&dissimilar),
        vec![(Some(1), Some(1)), (Some(2), Some(2)), (None, Some(3))]
    );
}

#[test]
fn repeated_lines_choose_earliest_monotonic_partners_deterministically() {
    // Structured inputs can contain equal lines in a changed run; textual comparison normally
    // separates such lines into context. Every source line still appears exactly once.
    let document = replacement(
        &["repeat old\n", "repeat old\n"],
        &["unrelated\n", "repeat new\n", "repeat new\n"],
    );
    let expected = vec![(None, Some(1)), (Some(1), Some(2)), (Some(2), Some(3))];
    for _ in 0..4 {
        assert_eq!(pairs(&document), expected);
    }
}

#[test]
fn similarity_anchors_do_not_cross_even_when_a_later_line_is_a_better_local_match() {
    let document = replacement(
        &["alpha beta gamma\n", "alpha beta delta\n"],
        &[
            "alpha beta theta\n",
            "alpha beta gamma\n",
            "alpha beta delta\n",
        ],
    );
    assert_eq!(
        pairs(&document),
        vec![(None, Some(1)), (Some(1), Some(2)), (Some(2), Some(3))]
    );
}

#[test]
fn long_and_oversized_runs_use_source_order_without_losing_source_lines() {
    let long = format!("{}\n", "界".repeat(6000));
    let document = replacement(
        &[&long, "setting: old\n"],
        &["unrelated\n", &long, "setting: new\n"],
    );
    assert_eq!(
        pairs(&document),
        vec![(Some(1), Some(1)), (Some(2), Some(2)), (None, Some(3))]
    );
    let old: Vec<_> = (0..128).map(|i| format!("setting {i:03}: old\n")).collect();
    let mut new = vec!["unrelated\n".to_string()];
    new.extend((0..128).map(|i| format!("setting {i:03}: new\n")));
    let old: Vec<_> = old.iter().map(String::as_str).collect();
    let new: Vec<_> = new.iter().map(String::as_str).collect();
    let document = replacement(&old, &new);
    let result = pairs(&document);
    assert_eq!(result.len(), 129);
    assert_eq!(result[0], (Some(1), Some(1)));
    assert_eq!(result[128], (None, Some(129)));
}

#[test]
fn candidate_work_budget_falls_back_before_the_line_and_byte_limits() {
    let old: Vec<_> = (0..20)
        .map(|i| format!("setting {i:03}: {} old\n", "a".repeat(80)))
        .collect();
    let mut new = vec!["unrelated\n".to_string()];
    new.extend((0..20).map(|i| format!("setting {i:03}: {} new\n", "a".repeat(80))));
    let old: Vec<_> = old.iter().map(String::as_str).collect();
    let new: Vec<_> = new.iter().map(String::as_str).collect();
    let document = replacement(&old, &new);
    assert_eq!(pairs(&document)[0], (Some(1), Some(1)));
}

#[test]
fn global_alignment_preserves_two_good_partners_instead_of_one_greedy_exact_match() {
    let document = replacement(
        &[
            "rule: retain source context for every request alpha\n",
            "rule: retain source context for every request beta\n",
        ],
        &[
            "rule: retain source context for every request changed alpha\n",
            "rule: retain source context for every request alpha\n",
        ],
    );
    assert_eq!(
        pairs(&document),
        vec![(Some(1), Some(1)), (Some(2), Some(2))]
    );
}

#[test]
fn aligned_unicode_retains_hit_testing_selection_search_and_resize_identity() {
    let document = replacement(
        &["label: cafe\u{301} 界 👩‍💻 old\n"],
        &["unrelated\n", "label: cafe\u{301} 世界 👩‍💻 new\n"],
    );
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 2,
    };
    let mut state = DiffState::new();
    for (width, wrap) in [(100, false), (24, true), (100, false)] {
        draw(&document, &mut state, width, wrap);
        let row = (0..state.row_count())
            .find(|&r| state.source_at(r, Side::New) == Some(position))
            .unwrap();
        assert_eq!(state.source_at(row, Side::Old).unwrap().line, 1);
        assert!(state.scroll_to_source(position));
        draw(&document, &mut state, width, wrap);
    }
    state.set_search(&document, "👩‍💻", Some(Side::New));
    assert_eq!(state.search_matches()[0].position, position);
    assert!(state.next_match());
    draw(&document, &mut state, 100, false);
    let row = (0..state.row_count())
        .find(|&r| state.source_at(r, Side::New) == Some(position))
        .unwrap();
    assert!(
        matches!(state.hit_test(54, row as u16), Some(HitTest::Source { new: Some(range), .. }) if range.position == position)
    );
    let selection = SourceSelection {
        anchor: SourceBoundary { position, byte: 7 },
        focus: SourceBoundary { position, byte: 13 },
    };
    assert!(state.set_selection(&document, selection));
    assert_eq!(
        state.selected_text(&document).as_deref(),
        Some("cafe\u{301}")
    );
}

#[test]
fn explicit_highlights_do_not_change_pairing_or_get_overwritten() {
    let document = replacement(&["setting: old\n"], &["unrelated\n", "setting: new\n"]);
    let mut files = document.files().to_vec();
    files[0].hunks[0].lines[0].highlights = Some(vec![]);
    let supplied = DiffDocument::new(files).unwrap();
    assert_eq!(pairs(&supplied), vec![(None, Some(1)), (Some(1), Some(2))]);
    assert_eq!(
        supplied.files()[0].hunks[0].lines[0].highlights,
        Some(vec![])
    );
}

#[test]
fn file_and_context_reveal_resolve_original_sources_after_alignment() {
    let prefix: String = (1..=12).map(|i| format!("context {i:02}\n")).collect();
    let old = format!("{prefix}setting: old\n");
    let new = format!("{prefix}unrelated\nsetting: new\n");
    let document = DiffDocument::compare(&old, &new, usize::MAX);
    let widget = Diff::new(&document)
        .mode(ViewMode::Split)
        .context_lines(Some(0));
    let area = Rect::new(0, 0, 80, 20);
    let mut state = DiffState::new();
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer, &mut state);
    let fold = state.file_folds()[0];
    assert!(state.set_file_expanded(&fold, false));
    widget.render(area, &mut buffer, &mut state);
    let position = SourcePosition {
        file: 0,
        side: Side::New,
        line: 14,
    };
    assert!(state.scroll_to_source(position));
    widget.render(area, &mut buffer, &mut state);
    let row = (0..state.row_count())
        .find(|&r| state.source_at(r, Side::New) == Some(position))
        .unwrap();
    assert_eq!(state.source_at(row, Side::Old).unwrap().line, 13);
    assert!(state.scroll_to_source(SourcePosition {
        line: 5,
        ..position
    }));
    widget.render(area, &mut buffer, &mut state);
    assert!(
        state
            .context_folds()
            .iter()
            .any(|fold| state.context_expanded(fold))
    );
}
