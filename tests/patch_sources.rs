//! Patch text as emitted by common version-control and diff tools.
use ratatui_diff::{DiffDocument, LineKind};

fn summary(document: &DiffDocument) -> Vec<(Option<&str>, Option<&str>, Vec<LineKind>)> {
    document
        .files()
        .iter()
        .map(|file| {
            let kinds = file
                .hunks
                .iter()
                .flat_map(|hunk| hunk.lines.iter().map(|line| line.kind))
                .collect();
            (file.old_path.as_deref(), file.new_path.as_deref(), kinds)
        })
        .collect()
}

const CHANGE: [LineKind; 2] = [LineKind::Delete, LineKind::Insert];

#[test]
fn commit_headers_before_git_diffs_are_not_patches() {
    let jj = "Commit ID: b8c039ec65271dcd2d735110a79994776c6fc7bb\n\
              Change ID: kyulqtyxunxvxwmwvwolnkoovkttvpwk\n\
              Author   : A U Thor <author@example.com> (2026-10-14 13:38:20)\n\
              \n    Describe the change\n\n\
              diff --git a/f b/f\nindex 1234567..7654321 100644\n--- a/f\n+++ b/f\n\
              @@ -1 +1 @@\n-old\n+new\n";
    let document = DiffDocument::parse(jj).unwrap();
    assert_eq!(
        summary(&document),
        [(Some("f"), Some("f"), CHANGE.to_vec())]
    );
    assert_eq!(document.files()[0].metadata[0], "diff --git a/f b/f");
}

#[test]
fn format_patch_mail_is_not_metadata() {
    let mail = "From b5ec6e7ca5f3736f2f611d172378c5de78ac502a Mon Sep 17 00:00:00 2001\n\
                From: A U Thor <author@example.com>\n\
                Subject: [PATCH] Describe the change\n\
                \nBody text.\n---\n f | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\n\
                diff --git a/f b/f\nindex 1234567..7654321 100644\n--- a/f\n+++ b/f\n\
                @@ -1,2 +1,2 @@\n-old\n+new\n same\n-- \n2.56.0\n\n";
    let document = DiffDocument::parse(mail).unwrap();
    let kinds = [LineKind::Delete, LineKind::Insert, LineKind::Context];
    assert_eq!(summary(&document), [(Some("f"), Some("f"), kinds.to_vec())]);
    assert!(
        document.files()[0]
            .metadata
            .iter()
            .all(|m| !m.contains("Subject"))
    );
}

#[test]
fn git_log_commit_headers_between_diffs_are_skipped() {
    let log = "commit 1111111111111111111111111111111111111111\n\
               Author: A U Thor <author@example.com>\n\n    Second\n\n\
               diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-b\n+c\n\n\
               commit 2222222222222222222222222222222222222222\n\
               Author: A U Thor <author@example.com>\n\n    First\n\n\
               diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n";
    let document = DiffDocument::parse(log).unwrap();
    assert_eq!(document.files().len(), 2);
    assert!(document.files().iter().all(|file| file.hunks.len() == 1));
}

#[test]
fn unified_tool_headers_parse_as_written() {
    let gnu = "--- f\t2026-10-14 10:00:00.000000000 +0000\n\
               +++ f\t2026-10-14 10:01:00.000000000 +0000\n@@ -1 +1 @@\n-old\n+new\n";
    let hg = "diff -r 1234abcd f\n--- a/f\tThu Jan 01 00:00:00 1970 +0000\n\
              +++ b/f\tThu Jan 01 00:00:00 1970 +0000\n@@ -1 +1 @@\n-old\n+new\n";
    let svn = "Index: f\n===================================================================\n\
               --- f\t(revision 1)\n+++ f\t(working copy)\n@@ -1 +1 @@\n-old\n+new\n";
    for (patch, old, new) in [(gnu, "f", "f"), (hg, "f", "f"), (svn, "f", "f")] {
        let document = DiffDocument::parse(patch).unwrap();
        assert_eq!(
            summary(&document),
            [(Some(old), Some(new), CHANGE.to_vec())]
        );
    }
}

#[test]
fn non_unified_formats_are_rejected() {
    let context = "*** f\t2026-10-14 10:00:00\n--- f\t2026-10-14 10:01:00\n\
                   ***************\n*** 1 ****\n! old\n--- 1 ----\n! new\n";
    let error = DiffDocument::parse(context).unwrap_err();
    assert_eq!(error.offset(), Some(0));
    assert!(error.message().contains("context diffs"), "{error}");

    assert!(DiffDocument::parse("1c1\n< old\n---\n> new\n").is_err());
    assert!(DiffDocument::parse("Modified regular file f:\n   1    1: oldnew\n").is_err());
}

#[test]
fn git_prefixes_are_removed_from_every_operation() {
    let patch = "diff --git a/m b/m\n--- a/m\n+++ b/m\n@@ -1 +1 @@\n-a\n+b\n\
                 diff --git a/c b/c\nnew file mode 100644\n--- /dev/null\n+++ b/c\n@@ -0,0 +1 @@\n+x\n\
                 diff --git a/d b/d\ndeleted file mode 100644\n--- a/d\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n\
                 diff --git a/x b/y\nsimilarity index 100%\nrename from x\nrename to y\n";
    let document = DiffDocument::parse(patch).unwrap();
    let paths: Vec<_> = document
        .files()
        .iter()
        .map(|file| (file.old_path.as_deref(), file.new_path.as_deref()))
        .collect();
    assert_eq!(
        paths,
        [
            (Some("m"), Some("m")),
            (None, Some("c")),
            (Some("d"), None),
            (Some("x"), Some("y")),
        ]
    );
    let unprefixed = "diff --git a/f a/f\n--- a/f\n+++ a/f\n@@ -1 +1 @@\n-a\n+b\n";
    let document = DiffDocument::parse(unprefixed).unwrap();
    assert_eq!(document.files()[0].new_path.as_deref(), Some("a/f"));
}

#[test]
fn plain_unified_files_keep_their_own_headers() {
    let svn = "Index: f\n===\n--- f\n+++ f\n@@ -1 +1 @@\n-a\n+b\n\
               Index: g\n===\n--- g\n+++ g\n@@ -1,2 +1,2 @@\n same\n-a\n+b\n\\ No newline at end of file\n";
    let document = DiffDocument::parse(svn).unwrap();
    let metadata: Vec<_> = document.files().iter().map(|file| &file.metadata).collect();
    assert_eq!(metadata, [&["Index: f", "==="], &["Index: g", "==="]]);
    assert_eq!(document.files()[1].hunks[0].lines.len(), 3);
}

#[test]
fn parse_errors_point_at_the_malformed_hunk() {
    let patch = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n\
                 diff --git a/g b/g\nindex 1..2 100644\n--- a/g\n+++ b/g\n@@ -1,2 +1,2 @@\n-a\n+b\n";
    let error = DiffDocument::parse(patch).unwrap_err();
    assert_eq!(error.offset(), patch.rfind("@@ "));
    assert_eq!(error.message(), "hunk header does not match hunk");

    let plain =
        "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n--- a/g\n+++ b/g\n@@ -1,2 +1,2 @@\n-a\n+b\n";
    assert_eq!(
        DiffDocument::parse(plain).unwrap_err().offset(),
        plain.rfind("@@ ")
    );
}

#[test]
fn lines_beyond_a_hunk_count_are_rejected() {
    let patch = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n+extra\n";
    let error = DiffDocument::parse(patch).unwrap_err();
    assert_eq!(error.offset(), patch.find("+extra"));
    assert!(error.message().contains("more lines"), "{error}");
}

#[test]
fn hunk_headings_are_kept_and_rendered() {
    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;
    use ratatui_core::widgets::StatefulWidget;
    use ratatui_diff::{Diff, DiffState};

    let patch = "--- a/f\n+++ b/f\n@@ -4 +4 @@ fn retry() {\r\n-a\n+b\n@@ -9 +9 @@\n-c\n+d\n";
    let document = DiffDocument::parse(patch).unwrap();
    let hunks = &document.files()[0].hunks;
    assert_eq!(hunks[0].heading.as_deref(), Some("fn retry() {"));
    assert_eq!(hunks[1].heading, None);

    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    let mut state = DiffState::new();
    let diff = Diff::new(&document);
    diff.render(area, &mut buffer, &mut state);
    let row: String = (0..area.width).map(|x| buffer[(x, 2)].symbol()).collect();
    assert_eq!(row.trim_end(), "@@ -4,1 +4,1 @@ fn retry() {");

    let mut files = document.files().to_vec();
    files[0].hunks[0].heading = Some("two\nlines".into());
    assert!(DiffDocument::new(files).is_err());
}
