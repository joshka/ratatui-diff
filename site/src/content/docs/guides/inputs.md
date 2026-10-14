---
title: Choose an input
description: Build a DiffDocument from two strings, a Git patch, or structured hunks.
---

Every diff starts as a `DiffDocument`. Pick the constructor that matches what your application
already has. Documents are immutable and can be shared by several views.

| You have                        | Use                                     | Context available                      |
| ------------------------------- | --------------------------------------- | -------------------------------------- |
| Old and new text                | `DiffDocument::from_text(old, new)`     | Full source; three context lines shown |
| Old and new text, custom hunks  | `DiffDocument::compare(old, new, n)`    | Full source; `n` context lines         |
| A unified or multi-file patch   | `DiffDocument::parse(patch)`            | Only lines present in the patch        |
| Your own files, hunks, or lines | `DiffDocument::new(files)`              | Whatever you supply, validated         |

## Compare two strings

```rust
use ratatui_diff::DiffDocument;

let document = DiffDocument::from_text("timeout_ms: 500\n", "timeout_ms: 1500\n");
```

`from_text` compares UTF-8 source with three context lines around each change. Use `compare` to pick
another radius. To keep every unchanged line available for [context folding](../folding/), pass
`usize::MAX`:

```rust
use ratatui_diff::DiffDocument;

let old = "a\nb\nc\n";
let new = "a\nB\nc\n";
let document = DiffDocument::compare(old, new, usize::MAX);
```

Identical inputs produce a document with no hunks.

## Parse a patch

```rust
use ratatui_diff::DiffDocument;

let patch = "--- a/greeting\n+++ b/greeting\n@@ -1 +1 @@\n-hello world\n+hello Rust\n";
let document = DiffDocument::parse(patch)?;
assert_eq!(document.files().len(), 1);
```

`parse` accepts unified diffs and Git's extended format, including renames, copies, created and
deleted files, binary files, and metadata-only changes. Each `diff --git` section is read as Git
output; other sections are read as plain unified diffs. Parse errors report byte offsets when
available.

| Source                                        | Accepted input                                           |
| --------------------------------------------- | -------------------------------------------------------- |
| Git                                           | `git diff`, `git show`, `git log -p`, `git format-patch` |
| jj                                            | `jj diff --git`, `jj show --git`                         |
| Mercurial, Subversion, `diff -u`              | Their unified output                                     |
| Context (`diff -c`), normal, or combined diff | Rejected                                                 |

Commit and mail headers before the first Git section are skipped. jj's default color-words output is
a display format rather than a patch, so pass `--git`. Paths are kept as written, so Git-style `a/`
and `b/` prefixes remain in `DiffFile` paths.

A patch only contains the lines it was generated with. The widget shows omitted regions as
unavailable rather than inventing them, and selection and search never cross those gaps. If your
host can read the full files, comparing them with `compare` gives the reader expandable context.

![Two files: the first was compared from full source and offers expandable unchanged lines; the second came from a patch and marks its missing context as unavailable.](/ratatui-diff/assets/context-folds.png)

## Supply structured input

`DiffDocument::new` validates caller-provided files, hunks, lines, and highlight ranges. Use it when
another tool already computed the diff, or to supply your own word highlights.

## Text fidelity

Source content keeps CRLF line endings and final-newline differences. Control characters are shown
as visible notation instead of being sent to the terminal. Binary changes appear as one-line
summaries, and metadata-only changes (such as a mode change) are labeled.

## Large inputs

Comparison is synchronous. For large documents, prepare the `DiffDocument` on a worker thread and
hand it to the UI when it is ready. Rendering reuses the comparison and only draws visible rows, but
the first layout and each resize process the whole document. Very large replacements fall back to
whole-line styling when they exceed the word-comparison limits. See
[Performance](../../reference/performance/) for measurements.
