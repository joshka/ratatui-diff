---
title: Appearance
description: Choose unified or split view, wrapping, whitespace, line numbers, and themes.
---

Every option on this page is a method on `Diff`, so you can change it from one frame to the next.
The viewport stays anchored to the nearest source line when layout changes.

All screenshots render the same change to a client's retry settings, using
`DiffTheme::aardvark_ink()` at 101 columns (49 for the narrow examples). They capture the widget
alone, without any application chrome.

| Option          | Default    | Method                    |
| --------------- | ---------- | ------------------------- |
| Layout          | Unified    | `.mode(ViewMode::Split)`  |
| Wrapping        | Off        | `.wrap(true)`             |
| Whitespace      | Hidden     | `.whitespace(true)`       |
| Tab width       | 4 cells    | `.tab_width(8)`           |
| Line numbers    | Shown      | `.line_numbers(false)`    |
| Word highlights | Shown      | `.word_highlights(false)` |
| Theme           | `dark()`   | `.theme(DiffTheme::…)`    |

## Unified or split

Unified view gives each line the full width, which suits narrow hosts and changes that read top to
bottom. Old and new line numbers sit in separate columns, and `-` and `+` mark each side.

```rust
use ratatui_diff::{Diff, DiffDocument, DiffTheme, ViewMode};

let document = DiffDocument::from_text("timeout_ms: 500,\n", "timeout_ms: 1500,\n");
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Unified);
```

![Unified view shows the removed retry settings immediately before their replacements.](/ratatui-diff/assets/unified.png)

Split view keeps old and new source side by side. Blank rows keep changes of different lengths
aligned. Replacements pair lines by similarity within bounds, preserving source order; moved code is
not detected.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split);
```

![Split view aligns the changed timeout and retry count; the added backoff line has no old counterpart.](/ratatui-diff/assets/split.png)

The widget never switches modes on its own. If your host wants a breakpoint, pick the mode from the
area width before rendering.

## Word highlights and line numbers

Word highlights point to small edits inside a changed line. Adjacent changed words join into one
highlighted phrase. Disable them to keep only the row colors:

```rust
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .word_highlights(false);
```

![With word highlights disabled, additions and deletions keep their row colors and markers.](/ratatui-diff/assets/lines-only.png)

Hiding line numbers recovers space for source. The `+` and `-` markers stay, so the change is still
visible without relying on color.

```rust
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .line_numbers(false);
```

![Without line numbers, plus and minus markers still distinguish the two sides.](/ratatui-diff/assets/no-numbers.png)

## Whitespace

Whitespace markers make indentation and trailing spaces visible without changing the source. The
generated dots and tab arrows are dimmed and keep the row's background. Literal dots in the source
keep their normal style, and copied text never contains markers.

```rust
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .whitespace(true);
```

![Visible space markers expose indentation in both panes.](/ratatui-diff/assets/whitespace.png)

## Narrow views and wrapping

Without wrapping, a split view clips long lines at the pane boundary and scrolls both panes
horizontally together (`state.scroll_horizontal(cells)`). Graphemes are never split at the edge.

![At 49 columns, unwrapped split lines clip at the pane boundary instead of running into the other pane.](/ratatui-diff/assets/narrow-split.png)

With wrapping, long lines continue on extra rows. Continuations show a dim `↪` in the number column,
and a split row takes the height of its taller side.

```rust
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .whitespace(true)
    .wrap(true);
```

![Wrapping keeps pane boundaries and marks continued lines with dim arrows.](/ratatui-diff/assets/narrow-wrapped.png)

Wrapping respects grapheme clusters and wide characters, including CJK text and emoji:

![Unicode fixture in split view before wrapping, with long routes clipped at the pane edge.](/ratatui-diff/assets/unicode-unwrapped.png)

![The same fixture wrapped, exposing the ends of the long routes without splitting wide characters.](/ratatui-diff/assets/unicode-wrapped.png)

## Themes

Each `DiffTheme` style can be customized independently, starting from one of four presets:

| Preset           | Colors                                       | Use it when                                 |
| ---------------- | -------------------------------------------- | ------------------------------------------- |
| `dark()`         | Terminal palette (ANSI)                      | Respecting the user's dark terminal palette |
| `light()`        | RGB text over terminal palette roles         | Running in a light terminal                 |
| `aardvark_ink()` | Explicit RGB foregrounds and backgrounds     | You want fixed colors on a dark surface     |
| `monochrome()`   | None; markers, bold, and underline only      | Color is unavailable or unwanted            |

`aardvark_ink()` needs truecolor support and also sets the widget background. Its source text,
headers, markers, and changed words keep at least 4.5:1 contrast against their row backgrounds.

Monochrome keeps `+` and `-` and underlines changed words, so the diff stays readable with no color
at all:

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::monochrome())
    .mode(ViewMode::Split)
    .whitespace(true)
    .wrap(true);
```

![Monochrome uses markers and underlines to preserve change information.](/ratatui-diff/assets/mono.png)

Switching themes or toggling word highlights doesn't rebuild layout, so it is cheap to do every
frame.

## Margins

The widget fills the area you give it. To add padding or room for controls, render it into a smaller
`Rect` from your own layout. Each source line always occupies one terminal row per displayed line.
