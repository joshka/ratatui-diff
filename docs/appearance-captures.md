# Appearance capture notes

Maintain the screenshots and local HTML for [Choosing a diff presentation](appearance.md) with
consistent source content, terminal settings, and framing. Capture the widget alone; keep
application toolbars and key-binding footers out of documentation images.

## Reproduce the guide

The checked-in tapes pin the complete Aardvark Ink terminal palette. Captures were reviewed with
Betamax 0.1.21. Install that version and JetBrains Mono; `pandoc` renders the standalone local
guide. The source images and generated HTML stay in ignored `media/`. Publish reviewed images
separately using the [release workflow](releasing.md).

```sh
just appearance
```

Open `media/appearance.html` for the visual guide with embedded images. Run the interactive example
with `cargo run --example viewer -- --aardvark-ink`.

## Design references and remaining coverage

[GitHub Primer](https://primer.style/product/getting-started/react/theme-reference/) separates diff
line, word, and line-number colors. That separation is useful here: increased emphasis should mark
increasingly specific changes, while the gutter stays legible.

[Pierre's Diffs](https://diffs.com/#styles) places controls beside examples so a reader can see what
an option changes. Its
[stylesheet](https://github.com/pierrecomputer/pierre/blob/main/packages/diffs/src/style.css)
derives row tints from the base and change colors, with separate emphasis colors. Its split examples
also distinguish empty alignment cells from source. Use the same option-to-result structure for new
captures.

[Delta](https://dandavison.github.io/delta/custom-themes.html) keeps appearance configuration
separate from options such as side-by-side view. `DiffTheme` and `ViewMode` make the same useful
distinction.

For the README, use one clear split or unified image near the introduction and link to the
appearance guide for option comparisons. Keep the API lifecycle concise there; retain exact
contracts in Rustdoc. Before a future website, capture these additional cases using the same font,
framing, and base:

- Terminal-owned dark colors and a genuinely light terminal, each with a tested host palette.
- Tabs, trailing spaces, Unicode graphemes, and final-newline changes in focused fixtures.
- Several files, long paths, separated hunks, and metadata-only changes.
- Several viewport heights and runtime resizing anchored to the same source line.

Each image should answer one question, name the option, and use a short original change a reader can
understand. Avoid giant fixtures, decorative terminal chrome, tiny text, and galleries that change
both the data and settings between comparisons.

## Focused visual regression review

Use the existing client-settings fixture for the even-width edge and narrow interaction checks:

```sh
cargo build --example viewer --locked
python3 scripts/capture.py examples/aardvark-ink-even.tape
python3 scripts/capture.py examples/appearance-interaction.tape
```

The even-width tape captures 100 columns, including the final padding cell on added rows. The
interaction tape captures clipping, synchronized horizontal scrolling, wrapping with visible
whitespace, scrolling to the end, mode changes, and a monochrome round trip in a short viewport.
Resize/source-anchor and widget-boundary checks also run in the rendered-buffer tests; the tapes use
fixed terminal geometry.

Before editing presentation, copy baseline images to unique names in `media/`. Give each iteration
its own screenshot paths so review links retain their original pixels. Inspect both the whole frame
and changed cells. Keep neutral empty partners, readable gutters, complete graphemes, and non-color
change cues. Stop when further adjustments only substitute equivalent colors or add visual noise.

## Remaining header work

The current theme shares one style between file and hunk headers. Review separate header styles and
old/new pane labels with narrow captures before extending the API.

## Framing and code colors

`--capture VARIANT` in the viewer example renders only the selected widget. The capture script asks
`--measure VARIANT COLUMNS` for its displayed row count before sizing the terminal. Captures use 101
or 49 columns, a 22-pixel JetBrains Mono font, and 22 pixels of padding on every side. The pinned
font settings yield 14-pixel columns and 22-pixel rows; changing them requires updating the pixel
conversion in `scripts/capture-appearance.py`. Odd column counts fit two equal split panes and the
one-column separator. At even widths, the spare rightmost cell uses the right row's background as
padding; it does not add a source column.

Code blocks use the syntax colors from
[VS Code Dark 2026](https://github.com/microsoft/vscode/blob/main/extensions/theme-defaults/themes/2026-dark.json)
on an ink-colored surface. Pandoc token classes approximate the theme's TextMate roles; the diff
widget itself has no syntax highlighting. Adjust the `--code-*` variables in
`scripts/appearance.css` to tune the guide without changing diff colors. For a closer Aardvark
match, try `--code-keyword: #e48383` and `--code-function: #d58bf0`.
