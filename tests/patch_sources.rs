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
        [(Some("a/f"), Some("b/f"), CHANGE.to_vec())]
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
    assert_eq!(
        summary(&document),
        [(Some("a/f"), Some("b/f"), kinds.to_vec())]
    );
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
    for (patch, old, new) in [(gnu, "f", "f"), (hg, "a/f", "b/f"), (svn, "f", "f")] {
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
    assert_eq!(error.offset, Some(0));
    assert!(error.message.contains("context diffs"), "{error}");

    assert!(DiffDocument::parse("1c1\n< old\n---\n> new\n").is_err());
    assert!(DiffDocument::parse("Modified regular file f:\n   1    1: oldnew\n").is_err());
}
