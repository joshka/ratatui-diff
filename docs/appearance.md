<!-- rumdl-disable MD057 -->
<!-- Images are generated locally by `just appearance`; release assets are not committed. -->

# Choosing a diff presentation

Compare layouts and display options using a change to a client's retry settings: a longer timeout,
more retries, backoff, and support for rate-limited requests. All screenshots use the same source
from the [interactive example](../examples/viewer.rs).

![Split view aligns the changed timeout and retry count; the added backoff line has no old counterpart.](../media/aardvark-split.png)

## Choose a theme

`DiffTheme::aardvark_ink()` supplies an explicit dark RGB surface, green additions, and red
deletions. Row backgrounds show the extent of each change; stronger backgrounds and bold text
identify changed words. Dimmed neutral line numbers remain readable on either row color. Cyan file
and hunk headers use the same ink background as the source.

```rust
use ratatui_diff::{Diff, DiffDocument, DiffTheme, ViewMode};

let document = DiffDocument::from_text("timeout_ms: 500,\n", "timeout_ms: 1500,\n");
let diff = Diff::new(&document)
    .mode(ViewMode::Split)
    .theme(DiffTheme::aardvark_ink());
```

Choose `aardvark_ink()` for fixed RGB colors, or `dark()` to use the terminal's ANSI palette. The
RGB preset requires truecolor support and sets the widget background as well as its text colors. The
example terminal also uses Aardvark Ink for the surrounding screen.

| Preset           | Color source                                   | Use it when                                |
| ---------------- | ---------------------------------------------- | ------------------------------------------ |
| `dark()`         | Terminal palette                               | Respecting a dark terminal palette         |
| `light()`        | RGB text overrides with terminal palette roles | Using a light terminal palette             |
| `aardvark_ink()` | Explicit RGB foregrounds and backgrounds       | Using fixed colors on a dark background    |
| `monochrome()`   | No explicit colors                             | Displaying changes without explicit colors |

The default is `dark()`. `light()` overrides text colors and inherits other styles from `dark()`;
check its headers and word highlights against the terminal palette used by your application.

## Choose unified or split view

Unified view gives each line the full width. It suits narrow hosts and changes that read naturally
from top to bottom. Old and new line numbers stay separate; `-` and `+` identify the source side.
One space separates numbers from the change marker; source text follows the marker immediately.
Numbers and the pane separator recede while change markers use the source row's color.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Unified);
```

![Unified view shows the removed retry settings immediately before their replacements.](../media/aardvark-unified.png)

Split view keeps old and new source adjacent, with blank rows aligning changes of different lengths.
Replacements pair in source order; moved code is not detected.

## Adjust word highlights and line numbers

Word highlights help locate small edits inside a changed line. Automatic highlights join changed
words across intervening whitespace, forming one highlighted phrase. Leading and trailing unchanged
whitespace stays outside the phrase; caller-supplied ranges remain exact. Disable word highlights to
show only row colors.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split)
    .word_highlights(false);
```

![With word highlights disabled, additions and deletions retain their row colors and markers.](../media/aardvark-lines-only.png)

Line numbers can be hidden independently. The change markers remain, preserving a non-color cue and
recovering space for source text.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split)
    .line_numbers(false);
```

![Without line numbers, plus and minus markers still distinguish the two sides.](../media/aardvark-no-numbers.png)

## Inspect whitespace

Whitespace markers make indentation and spaces visible without changing the source. Generated dots
and tab arrows are dimmed, retaining the row and word-highlight backgrounds. Literal dots and arrows
in source keep their text style.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split)
    .whitespace(true);
```

![Visible space markers expose indentation in both panes.](../media/aardvark-whitespace.png)

## Fit narrow views

An unwrapped split view clips long lines at the pane boundary. Horizontal scrolling is synchronized.
Wrapping breaks long lines across additional rows. The first row shows the source number;
continuations show `↪`, dimmer than the numbers, in the number column. Empty alignment cells stay
blank. Switching to unified view is another useful host choice. The widget does not switch modes
automatically at a breakpoint.

![At a narrow width, unwrapped split lines clip rather than running into the neighboring pane.](../media/aardvark-narrow-split.png)

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split)
    .whitespace(true)
    .wrap(true);
```

![Wrapping preserves pane boundaries and marks continued source lines with dim arrows.](../media/aardvark-narrow-wrapped.png)

## Use monochrome

Monochrome retains `+` and `-`, with underlines for changed words. The host terminal supplies the
foreground and background. Switching themes preserves the viewport.

```rust
let diff = Diff::new(&document)
    .theme(DiffTheme::monochrome())
    .mode(ViewMode::Split)
    .whitespace(true)
    .wrap(true);
```

![Monochrome uses markers and underlines to preserve change information.](../media/aardvark-mono.png)

## Add space around the widget

Render the widget inside a smaller rectangle to leave room for margins and controls. Source line
spacing remains one terminal row per displayed line; padding belongs to the application layout.

The interactive example leaves room for its controls. Documentation images render only the widget,
with 22 pixels (one character height) of image padding on each side. Each image's height follows its
displayed row count, including wrapped lines.

## Palette and verification

| Role         | Foreground | Row background | Changed-word background |
| ------------ | ---------- | -------------- | ----------------------- |
| Context      | `#b4bcca`  | `#0f141f`      | —                       |
| Addition     | `#75cf84`  | `#162b25`      | `#254835`               |
| Deletion     | `#e48383`  | `#30202a`      | `#4b2530`               |
| Header       | `#52c4c0`  | `#0f141f`      | —                       |
| Gutter       | `#6f7a8f`  | Inherits row   | —                       |

The source foregrounds and base come from the Aardvark Ink Ghostty palette; the dimmed gutter and
tinted change backgrounds are chosen for this widget. Rendered-cell tests require at least 4.5:1
foreground/background contrast for source text, headers, change markers, and changed words. The
quieter number and separator colors retain at least 3:1 contrast against their row backgrounds. With
explicit RGB foregrounds and backgrounds, whitespace markers retain one-quarter of the
foreground-to-background channel difference; continuation arrows retain one-half of the
number-to-background difference. These blends preserve each cell's background and word modifiers.
They establish a consistent hierarchy, not a perceptual contrast standard. When either color is
terminal-owned, synthetic cues use the terminal's dim modifier. Change markers and underlines
preserve cues without color. Terminal rendering and font settings can still affect readability.

## Run the example

```sh
cargo run --example viewer -- --aardvark-ink
```

Use `s` for split view, `w` for wrapping, `t` for whitespace markers, `n` for line numbers, `i` for
word highlights, and `m` for monochrome. Arrow keys scroll; `q` exits.

To regenerate the images and standalone HTML, see the
[capture notes](../docs/appearance-captures.md).
