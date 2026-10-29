---
title: Syntax colors
description: Add language-aware foreground colors with the optional syntax feature.
---

Syntax colors are optional. Enable the `syntax` feature to get bundled
[Syntect](https://github.com/trishume/syntect) grammars and
[two-face](https://github.com/CosmicHorrorDev/two-face) themes. You pick a language and theme; the
library tokenizes the source and maps colors onto the diff's original bytes.

```sh
cargo add ratatui-diff --features syntax
```

Default builds don't include these dependencies. Native Oniguruma compilation needs a C toolchain.

## Before and after

Without syntax colors, context lines use the theme's plain foreground:

![A Rust change in split view with diff colors only.](/ratatui-diff/assets/syntax-off.png)

With the `Coldark-Dark` theme, adjusted for contrast against Aardvark Ink:

![The same change with keywords, types, strings, and numbers colored by the Coldark-Dark syntax theme.](/ratatui-diff/assets/syntax-split.png)

Syntax colors sit underneath diff styling, so row backgrounds and changed-word emphasis stay intact
in unified view too:

![Unified view with syntax colors under the deletion and insertion row backgrounds.](/ratatui-diff/assets/syntax-unified.png)

## Prepare, then attach

Preparation tokenizes the source and is the expensive step. Do it outside the draw callback, keep
the resulting `SyntaxStyles`, and borrow it from each frame's `Diff`.

```rust
use ratatui_diff::{Diff, DiffDocument, DiffTheme, FileSyntax, SyntaxHighlighter, SyntaxSource};

let old = "fn retry() { let timeout = 500; }\n";
let new = "fn retry() { let timeout = 1500; }\n";
let document = DiffDocument::from_text(old, new);
let theme = DiffTheme::aardvark_ink();
let highlighter = SyntaxHighlighter::bundled("Coldark-Dark")?.contrast_with(theme)?;
let styles = highlighter.prepare(
    &document,
    &[FileSyntax {
        file: 0,
        language: "rs",
        source: SyntaxSource::Full { old, new },
    }],
)?;
let diff = Diff::new(&document).theme(theme).syntax_styles(&styles)?;
```

`language` accepts a file extension or a syntax name. `SyntaxHighlighter::themes()` and
`SyntaxHighlighter::languages()` list the bundled choices. Files you leave out of the request render
without syntax colors.

`SyntaxStyles` owns its output. You can drop the source strings and the highlighter after
preparation. Styles attach only to the document they were prepared for (or a clone of it). Replacing
the styles, for example after a theme change, keeps scrolling, selection, search, and folds.

## Full or retained source

| Source                                | Accuracy                                                         |
| ------------------------------------- | ---------------------------------------------------------------- |
| `SyntaxSource::Full { old, new }`     | Parses omitted lines too, so multiline comments and strings work |
| `SyntaxSource::Retained`              | Best effort; parsing resets at each gap in the document          |

Use `Full` when you have the complete files. `Retained` is for patch-only input, where a hunk can
start inside a comment or string the parser never saw.

## Contrast

Bundled themes keep their own foreground colors by default. `contrast_with(theme)` adjusts any
foreground that falls under 4.5:1 contrast against the diff's context, insertion, deletion, and
changed-word backgrounds, mixing it toward white or black. Colors that already pass are unchanged.
Syntax backgrounds and underlines are discarded; bold and italic are kept. Search and selection
overlays still draw on top.

## Limits

By default, preparation accepts up to 64 MiB of input, 64 KiB per line, and one million retained
spans. Use `SyntaxLimits` for stricter bounds. Limits bound memory and input size, not regex time,
so schedule preparation for large inputs on a worker.

## Licenses

The bundled grammars and themes carry their own license terms. If you distribute an application with
the `syntax` feature, include the text returned by `SyntaxHighlighter::acknowledgements()`. The
viewer prints it with `cargo run --example viewer --features syntax -- --syntax-licenses`.
