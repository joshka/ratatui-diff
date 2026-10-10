//! Statistics are presentation-only and independent of folded source rows.
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffFile, DiffState, DiffTheme, HitTest, ViewMode};

fn document() -> DiffDocument {
    let mut text = DiffDocument::from_text("old\n", "new\nextra\n").files()[0].clone();
    text.old_path = Some("+path−.rs".into());
    text.new_path = text.old_path.clone();
    let binary = DiffFile {
        old_path: Some("image.png".into()),
        new_path: Some("image.png".into()),
        metadata: vec![],
        binary: true,
        hunks: vec![],
    };
    let metadata = DiffFile {
        old_path: Some("script".into()),
        new_path: Some("script".into()),
        metadata: vec!["new mode 100755".into()],
        binary: false,
        hunks: vec![],
    };
    DiffDocument::new(vec![text, binary, metadata]).unwrap()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (buffer.area.x..buffer.area.right())
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn colored_counts_keep_header_background_and_paths() {
    let document = document();
    for theme in [
        DiffTheme::aardvark_ink(),
        DiffTheme::dark(),
        DiffTheme::light(),
        DiffTheme::monochrome(),
    ] {
        let area = Rect::new(2, 3, 70, 8);
        let mut state = DiffState::new();
        let mut buffer = Buffer::empty(area);
        (&Diff::new(&document).theme(theme)).render(area, &mut buffer, &mut state);
        let text = row(&buffer, 3);
        assert!(text.contains("+path−.rs · +2 −1"));
        // A sign in the path is a label, not a statistic.
        assert_eq!(buffer[(4, 3)].fg, theme.header.fg.unwrap_or_default());
        let plus = text.chars().position(|c| c == '·').unwrap() + 2;
        assert_eq!(
            buffer[(area.x + plus as u16, 3)].fg,
            theme.insert.fg.or(theme.header.fg).unwrap_or_default()
        );
        assert_eq!(
            buffer[(area.x + plus as u16, 3)].bg,
            theme.header.bg.unwrap_or_default()
        );
        assert_eq!(
            buffer[(area.x + plus as u16 + 3, 3)].fg,
            theme.delete.fg.or(theme.header.fg).unwrap_or_default()
        );
    }
}

#[test]
fn optional_pinned_summary_counts_closed_binary_and_metadata_files() {
    let document = document();
    let mut state = DiffState::new();
    for file in 0..document.files().len() {
        assert!(state.set_file_expanded(&document.file_fold(file).unwrap(), false));
    }
    let area = Rect::new(2, 3, 80, 6);
    for mode in [ViewMode::Unified, ViewMode::Split] {
        let mut buffer = Buffer::empty(area);
        (&Diff::new(&document).mode(mode).show_stats(true)).render(area, &mut buffer, &mut state);
        assert!(row(&buffer, 3).contains("3 files · +2 −1 · 1 binary · 1 metadata"));
        assert!(row(&buffer, 4).contains("▸ +path−.rs"));
        assert!(state.hit_test(2, 3).is_none());
        assert!(matches!(
            state.hit_test(2, 4),
            Some(HitTest::FileHeader { .. })
        ));
        assert_eq!(state.row_count(), 3);
        state.end();
    }
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&document)).render(area, &mut buffer, &mut state);
    assert!(row(&buffer, 3).contains("▸ +path−.rs"));
}

#[test]
fn tiny_summary_viewports_and_replacement_are_safe() {
    let document = document();
    let replacement = DiffDocument::from_text("", "one\n");
    let mut state = DiffState::new();
    for area in [
        Rect::new(0, 0, 0, 0),
        Rect::new(0, 0, 1, 1),
        Rect::new(0, 0, 5, 2),
    ] {
        (&Diff::new(&document).show_stats(true)).render(area, &mut Buffer::empty(area), &mut state);
    }
    let area = Rect::new(0, 0, 60, 4);
    let mut buffer = Buffer::empty(area);
    (&Diff::new(&replacement).show_stats(true)).render(area, &mut buffer, &mut state);
    assert!(row(&buffer, 0).contains("1 file · +1 −0"));
}

#[test]
fn pinned_totals_do_not_scroll_or_claim_source_hits() {
    let old: String = (0..40).map(|n| format!("old {n}\n")).collect();
    let new: String = (0..40).map(|n| format!("new {n}\n")).collect();
    let document = DiffDocument::from_text(&old, &new);
    let area = Rect::new(2, 3, 60, 5);
    let mut state = DiffState::new();
    let widget = Diff::new(&document).show_stats(true);
    let mut buffer = Buffer::empty(area);
    (&widget).render(area, &mut buffer, &mut state);
    let summary = row(&buffer, 3);
    state.scroll_pages(1);
    (&widget).render(area, &mut buffer, &mut state);
    assert_eq!(row(&buffer, 3), summary);
    assert_eq!(state.offset(), 4);
    assert!(state.hit_test(2, 3).is_none());
    assert!(matches!(state.hit_test(9, 4), Some(HitTest::Source { .. })));
}
