# Folding controls

Keep folding preferences in `DiffState`, and let the host choose mouse events, keyboard bindings, or
saved review preferences. Files start expanded. Context starts fully visible unless the widget uses
`context_lines(Some(count))`. Folding changes display only: source numbers, search matches, and
extracted bytes still refer to the immutable document.

| Operation           | Hidden content                                    | Control              | Availability                            |
| ------------------- | ------------------------------------------------- | -------------------- | --------------------------------------- |
| File folding        | Metadata, hunks, and source below one file header | `FileFold`           | Before or after rendering               |
| Context folding     | Retained unchanged lines inside a hunk            | `ContextFold`        | After rendering with a context radius   |
| Unavailable context | Text absent from the document                     | No expansion control | Host must supply a replacement document |

## Restore saved file preferences

Resolve saved paths against the current document rather than persisting handles or assuming indexes
stay the same across documents. `DiffDocument::file_fold(index)` returns a validated handle or
`None` for an absent file. Its `file()` getter identifies the current file index.

```rust
use ratatui_diff::{DiffDocument, DiffState};

let document = DiffDocument::from_text("old\n", "new\n");
let saved_collapsed_paths = ["new"];
let mut state = DiffState::new();
for (index, file) in document.files().iter().enumerate() {
    if file.new_path.as_deref().is_some_and(|path| saved_collapsed_paths.contains(&path)) {
        let handle = document.file_fold(index).unwrap();
        assert!(state.set_file_expanded(&handle, false));
    }
}
// The first render uses these preferences without showing an expanded frame first.
```

A fresh state binds queued preferences to the first handle's document. Further handles must belong
to that document. The first matching render retains those preferences; rendering a different
document clears them and binds state to the replacement. Cloned documents preserve identity. This
rule also applies to an unchanged request to keep a file expanded. Unknown or foreign handles return
`false` from the setter; valid unchanged requests return `true`.

## Control files and retained context

After rendering, `file_folds()` lists all file handles, and `context_folds()` lists context
candidates, including expanded context inside collapsed files. A host can apply bulk file controls
with a loop:

```rust
use ratatui_diff::DiffState;

fn collapse_files(state: &mut DiffState) {
    let files = state.file_folds().to_vec();
    for file in &files {
        assert!(state.set_file_expanded(file, false));
    }
}
```

Use `file_expanded` and `context_expanded` to query state and the corresponding setters to change
it. Actual changes invalidate hit-testing until redraw. File headers return `HitTest::FileHeader`
with their handle; collapsed context controls return `HitTest::Fold`. Neither provides source bytes.
Unavailable-context cues remain ordinary header hits.

Collapsing a file preserves its nested context preferences. A context setter can change a nested
fold while its file stays collapsed. Width, wrapping, mode, and context-radius changes preserve file
preferences. Changing the radius resets context preferences; replacing the document resets both.

## Reveal and navigation

Explicit source jumps, search navigation, and selection reveal open the containing file and context
fold on the next render. Setting a query or selection alone does not open files. Selection reveal
takes precedence over pending search reveal. Manual collapse cancels pending automatic reveal, so
the next frame does not immediately reopen the file; a subsequent explicit request can reveal it.

File navigation visits headers without expanding them. Hunk navigation opens the target hunk's file
and visits its original header. Collapsing content at the viewport moves to the containing file
header; changes to files before or after the current viewport preserve the current source anchor.

Selection extraction includes retained text hidden by either fold. It rejects missing source gaps
and never includes header labels or fold summaries. Full retained context for differing generated
inputs is available through `DiffDocument::compare(old, new, usize::MAX)`; the default comparison
retains three context lines, and identical inputs still have no hunks. Retrieval of unavailable
source remains host work.

## Change counts and totals

File headers color additions and deletions with the theme's insertion/deletion foregrounds, while
keeping the header background. Monochrome keeps the `+` and `−` signs without requiring color.
Counts include supplied changed lines; wrapping and folding never change them. Binary payloads have
no line estimate, and metadata-only changes are labeled separately.

Use `Diff::new(&document).show_stats(true)` to pin a document-wide summary above the scrollable
content. It reserves one row and stays visible when files are collapsed or the diff is scrolled. The
option defaults to off. The summary has no source identity or hit target; the remaining rows form
the navigation viewport. The example's `d` key toggles totals.
