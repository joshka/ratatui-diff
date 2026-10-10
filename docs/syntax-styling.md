# Optional syntax highlighting

Enable the non-default `syntax` Cargo feature to use bundled Syntect grammars and two-face themes.
Callers select a language and theme; the library owns tokenization and mapping colors to source
bytes. Preparation happens outside rendering. Engine types and token spans remain private.

```rust
use ratatui_diff::{Diff, DiffDocument, DiffTheme, FileSyntax, SyntaxHighlighter, SyntaxSource};

let old = "let timeout = 500;\n";
let new = "let timeout = 1500;\n";
let document = DiffDocument::from_text(old, new);
let theme = DiffTheme::aardvark_ink();
let highlighter = SyntaxHighlighter::bundled("Coldark-Dark")?.contrast_with(theme)?;
let styles = highlighter.prepare(&document, &[FileSyntax {
    file: 0,
    language: "rs",
    source: SyntaxSource::Full { old, new },
}])?;
let widget = Diff::new(&document).theme(theme).syntax_styles(&styles)?;
# Ok::<(), ratatui_diff::SyntaxError>(())
```

## Sources and lifetime

`SyntaxSource::Full` validates retained lines against complete old and new snapshots, including line
endings. Each side has its own parser state; omitted prefixes are parsed to preserve multiline
comments and strings. Parsing stops after the last retained line. Size checks cover the entire
provided snapshot, including its tail.

`SyntaxSource::Retained` highlights only lines available in the document, resetting state at gaps.
This is best effort: a hunk can begin inside an omitted comment or string. Use complete snapshots
when lexical accuracy matters. Folded lines still participate in preparation.

`FileSyntax` identifies a document file by index and language by extension or supported syntax name.
Unknown languages/themes, duplicate files, source mismatches, engine failures, and exceeded limits
return errors. Files omitted from the request receive no syntax styles. Inspect `languages()` and
`themes()` for bundled choices.

`SyntaxStyles` owns its output and attaches only to the originating document or its clone. A freshly
constructed equivalent document is rejected. Source strings and the highlighter can be dropped after
preparation. Changing the palette requires preparing a replacement; attaching it preserves
scrolling, selection, search, folds, hit testing, and extraction. Hosts own worker scheduling and
stale-result rejection. Preparation is synchronous, with byte/span limits rather than a time limit.

## Color composition

Bundled RGB foregrounds are preserved by default. Syntax backgrounds and underlining are discarded;
bold and italic are retained. Non-opaque foregrounds, including terminal palette sentinels, are
rejected. Diff styles own context, insertion/deletion, and word backgrounds. Later whitespace,
search, and selection styling can override syntax foregrounds.

`contrast_with(theme)` explicitly adapts foregrounds to at least 4.5:1 contrast against the five
composed RGB source backgrounds: context, inserted/deleted lines, and inserted/deleted words.
Passing colors remain unchanged. Other colors are mixed toward white or black using the smallest
passing 8-bit fraction; white wins ties. This preserves channel order, not exact perceptual hue.
Unknown terminal backgrounds and palettes without a passing mixture return errors. The threshold
applies before whitespace dimming and interaction overlays, while using the same diff backgrounds.

Styles map to original byte ranges, preserving tabs, Unicode graphemes, wrapping, clipping, and
control-character display. If a grapheme crosses token boundaries, its first intersecting token
styles the whole displayed grapheme. Gutters, headers, markers, padding, and fold summaries do not
receive syntax styles. Rendering consumes prepared ranges without tokenizing or changing geometry.
Monochrome hosts can omit attachment; the example does so explicitly.

## Limits and dependencies

Defaults allow 64 MiB of input, 64 KiB per line including its newline, and one million coalesced
retained spans. `SyntaxLimits` allows hosts to set stricter bounds. These limits constrain
allocation and input volume; pathological grammar matching can still take time.

The optional dependency graph uses Syntect 5 and two-face 0.5 with the Oniguruma backend, matching
Codex's TUI manifests. Native Oniguruma compilation requires a C toolchain. External grammar, plist,
YAML, and dump loading are not exposed by this adapter. Default builds omit these dependencies.

Bundled assets include upstream license and attribution obligations. Distributors must retain
[the bundled notices](../LICENSE-syntax-assets.txt), also available through
`SyntaxHighlighter::acknowledgements()`. The example's `--syntax-licenses` prints them. Bincode
1.3.3 is a transitive bundled-asset decoder. Its unmaintained advisory has a narrow recorded
exception in `.config/deny.toml`; revisit that exception when Syntect migrates.

## Example and evidence

```sh
cargo run --example viewer --features syntax -- \
  --fixture syntax --aardvark-ink --syntax Coldark-Dark --syntax-contrast
```

Press `h` to toggle prepared colors. Omit `--syntax-contrast` to retain the original palette. Other
example fixtures use retained input; the syntax fixture supplies complete Rust snapshots. Tracked
before/after tapes cover unified, split, and narrow wrapped views, including combining characters
and emoji. Public integration tests verify source validation, independent side state, resource
limits, attachment identity, composed contrast, geometry, and overlay preservation. The
feature-gated Criterion group measures preparation separately from layout and scrolling.

See [syntax preparation and footprint](syntax-performance.md) for measured cold asset loading,
full/retained preparation, wrapped rendering, binary size, and resident memory with reproduction
commands and measurement limits.
