//! Bounded, deterministic inputs for exercising the interactive host.
use std::error::Error;

use ratatui_diff::{DiffDocument, DiffFile};

pub(super) fn load(name: &str) -> Result<DiffDocument, Box<dyn Error>> {
    match name {
        "showcase" => showcase(),
        "unicode" => unicode(),
        "unicode-text" => unicode_text(),
        "whitespace" => whitespace(),
        "multi-file" => multi_file(),
        _ => Err(format!(
            "unknown fixture: {name}; choose showcase, unicode, unicode-text, whitespace, or multi-file"
        )
        .into()),
    }
}

fn compared_file(path: &str, old: &str, new: &str, context: usize) -> DiffFile {
    let document = DiffDocument::compare(old, new, context);
    let mut file = document.files()[0].clone();
    file.old_path = Some(path.into());
    file.new_path = Some(path.into());
    file
}

fn unicode() -> Result<DiffDocument, Box<dyn Error>> {
    let old = "title: café / cafe\u{301} / 界\nstatus: 👩‍💻 reviewing 🦀\nroute: /資料/界界界界界界/very/long/component/name?query=alpha&language=日本語&status=pending\nmessage: keep graphemes together\n";
    let new = "title: café / cafe\u{301} / 世界\nstatus: 👩‍💻 approved ✅\nroute: /資料/界界界界界界/very/long/component/name?query=beta&language=日本語&status=approved\nmessage: keep graphemes together\nextra: emoji family 👨‍👩‍👧‍👦 and flag 🇯🇵\n";
    Ok(DiffDocument::new(vec![compared_file(
        "fixtures/unicode.txt",
        old,
        new,
        3,
    )])?)
}

fn unicode_text() -> Result<DiffDocument, Box<dyn Error>> {
    let old = "title: café / cafe\u{301} / 界\nroute: /資料/界界界界界界/very/long/component/name?query=alpha&language=日本語&status=pending\nmessage: keep graphemes together\n";
    let new = "title: café / cafe\u{301} / 世界\nroute: /資料/界界界界界界/very/long/component/name?query=beta&language=日本語&status=approved\nmessage: keep graphemes together\n";
    Ok(DiffDocument::new(vec![compared_file(
        "fixtures/unicode-text.txt",
        old,
        new,
        3,
    )])?)
}

fn whitespace() -> Result<DiffDocument, Box<dyn Error>> {
    let old = concat!(
        "section:\r\n",
        "    spaces:\r\n",
        "        name: old\r\n",
        "            values: one  two  \r\n",
        "\ttabs:\r\n",
        "\t\tname: old\r\n",
        "\t\t\tvalues: one\t two  \r\n",
        "    mixed:\r\n",
        "    \t    enabled: false\r\n",
        "final: no newline",
    );
    let new = concat!(
        "section:\r\n",
        "    spaces:\r\n",
        "        name: new\r\n",
        "            values: one  three \r\n",
        "\ttabs:\r\n",
        "\t\tname: new\r\n",
        "\t\t\tvalues: one\t three \r\n",
        "    mixed:\r\n",
        "    \t    enabled: true\r\n",
        "    \t\tadded: mixed indentation\r\n",
        "final: has newline\r\n",
    );
    Ok(DiffDocument::new(vec![compared_file(
        "fixtures/whitespace.txt",
        old,
        new,
        3,
    )])?)
}

fn multi_file() -> Result<DiffDocument, Box<dyn Error>> {
    let mut files = Vec::new();
    for (path, label) in [
        ("src/routes.txt", "route"),
        ("src/settings.txt", "setting"),
        ("docs/checklist.txt", "check"),
    ] {
        let mut old = String::new();
        let mut new = String::new();
        for line in 1..=48 {
            old.push_str(&format!("{label} {line:02}: ready\n"));
            match line {
                4 | 24 | 44 => {
                    new.push_str(&format!("{label} {line:02}: revised\n"));
                    new.push_str(&format!("{label} {line:02}: extra detail\n"));
                    new.push_str(&format!("{label} {line:02}: verified\n"));
                }
                _ => new.push_str(&format!("{label} {line:02}: ready\n")),
            }
        }
        files.push(compared_file(path, &old, &new, 3));
    }
    Ok(DiffDocument::new(files)?)
}

fn showcase() -> Result<DiffDocument, Box<dyn Error>> {
    // A small behavioral change keeps both whole-line and word-level edits visible.
    let old = r#"fn client_config() -> ClientConfig {
    ClientConfig {
        endpoint: "/v1/events",
        timeout_ms: 500,
        retries: 2,
    }
}

fn should_retry(status: u16) -> bool {
    status == 503
}
"#;
    let new = r#"fn client_config() -> ClientConfig {
    ClientConfig {
        endpoint: "/v1/events",
        timeout_ms: 1500,
        retries: 4,
        backoff: true,
    }
}

fn should_retry(status: u16) -> bool {
    matches!(status, 429 | 503)
}
"#;
    let compared = DiffDocument::compare(old, new, 3);
    let mut files = compared.files().to_vec();
    files[0].old_path = Some("src/client.rs".into());
    files[0].new_path = Some("src/client.rs".into());
    Ok(DiffDocument::new(files)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_viewports_wrap_search_and_scroll() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::widgets::StatefulWidget;
        use ratatui_diff::{Diff, DiffState, ViewMode};

        let document = load("unicode").unwrap();
        let area = Rect::new(0, 0, 47, 19);
        let mut buffer = Buffer::empty(area);
        let mut state = DiffState::new();
        let widget = Diff::new(&document).mode(ViewMode::Split);
        (&widget).render(area, &mut buffer, &mut state);
        let unwrapped_rows = state.row_count();
        (&widget.wrap(true)).render(area, &mut buffer, &mut state);
        assert!(state.row_count() > unwrapped_rows);

        let document = load("multi-file").unwrap();
        let widget = Diff::new(&document);
        (&widget).render(area, &mut buffer, &mut state);
        state.set_search(&document, "setting 24", None);
        assert_eq!(state.search_matches().len(), 4);
        state.end();
        (&widget).render(area, &mut buffer, &mut state);
        assert!(state.offset() > 0);
        assert!(state.offset() < state.row_count());
        state.start();
        (&widget).render(area, &mut buffer, &mut state);
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn fixtures_cover_distinct_source_contracts() {
        let document = load("multi-file").unwrap();
        assert_eq!(document.files().len(), 3);
        assert!(document.files().iter().all(|file| file.hunks.len() == 3));
        let document = load("unicode-text").unwrap();
        assert_eq!(document.files().len(), 1);
        let document = load("unicode").unwrap();
        let lines = &document.files()[0].hunks[0].lines;
        assert!(lines.iter().any(|line| line.text.contains("cafe\u{301}")));
        assert!(lines.iter().any(|line| line.text.contains("👨‍👩‍👧‍👦")));
        assert!(lines.iter().any(|line| line.text.len() > 100));
        let document = load("whitespace").unwrap();
        let lines = &document.files()[0].hunks[0].lines;
        assert!(lines.iter().any(|line| line.text.contains('\t')));
        assert!(lines.iter().any(|line| line.text.ends_with('\r')));
        assert!(lines.iter().any(|line| !line.terminated));
        assert!(load("unknown").is_err());
    }
}
