# Viewer Fixture Acceptance

Run `cargo run --example viewer --locked -- --help` to list the deterministic in-memory fixtures.
`showcase` is the default; existing `--aardvark-ink`, `--capture VARIANT`, and
`--measure VARIANT COLUMNS` commands retain it unless `--fixture NAME` is supplied. Fixture
selection can precede or follow those flags. Each fixture uses generated source strings; no files
are opened.

## Narrow Unicode and Long Lines

```sh
cargo run --example viewer --locked -- --fixture unicode --aardvark-ink
```

Use a terminal approximately 51 columns wide and 22 rows high.

1. Press `s`. Both panes show `fixtures/unicode.txt`; long lines clip within their panes.
1. Press `w`. Long routes wrap, exposing `language=日本語` and the changed status at their ends.
   Combining accents, wide characters, and joined emoji exercise grapheme boundaries.
1. Press `/`, type `approved`, then press Enter. The title reports matches; `f` and `F` move between
   the status and route matches. Press Esc to clear the search.
1. Press `v`, then Right, End, and `c`. The copy preview shows selected source text; wrapping does
   not insert display line breaks into the copied text. Press Esc to clear the selection.
1. Press `w` to disable wrapping; Right and Left pan the long lines. Press Home to return to the
   first row and `q` to quit.

The Betamax capture uses JetBrains Mono with the pinned Aardvark Ink palette. Some joined emoji and
flags lack glyphs in this font/rendering environment; inspect source/copy behavior separately from
font appearance. The tape captures before and after wrapping with identical geometry and input.

For a comparison without relying on emoji support, select `--fixture unicode-text`: it retains
combining accents, wide characters, and the long route while deliberately omitting emoji. The
widget-only detail tape compares `--capture whitespace` with `--capture wrapped`; visible whitespace
is enabled in both. These are separate source inputs, not cropped versions of the emoji scenario.

## Multiple Files, Hunks, and Uneven Replacements

```sh
cargo run --example viewer --locked -- --fixture multi-file --aardvark-ink
```

Use a terminal approximately 79 columns wide and 25 rows high. Three files each have three separated
hunks; every edited source line is replaced by three lines. Sources are bounded at 48 old lines and
54 new lines per file, with three context lines around each edit.

1. Press `]` three times. The viewport reaches the final routes hunk and the settings file header.
   Press `[` to move back. Repeated hunk jumps eventually reach `docs/checklist.txt`.
1. Press PageDown and PageUp to inspect intervening rows. End reaches the bottom; Home returns to
   the first file. Further movement at either boundary stays within the document.
1. Press `/`, type `setting 24`, then press Enter. The title reports four matches across the old and
   new sources. Press `s` and `w`: search remains active in split/wrapped presentation, and the
   extra replacement rows align against empty old-side rows. Use `f` and `F` to visit matches.
1. Press Esc, Home, `v`, Down, End, and `c` to inspect a multiline selection's copy preview. Press
   Esc to clear selection and `q` to quit.

## Tabs, CRLF, and Final Newlines

```sh
cargo run --example viewer --locked -- --fixture whitespace --aardvark-ink
```

1. Press `s` and `t`. Tabs appear as arrows, spaces as dots, and retained CR characters as `\u{d}`.
   The old final line has an unterminated-line marker; the new final line retains CRLF.
1. Press `w`, then `n`, `i`, and `m` to inspect wrapping, hidden numbers, disabled word highlights,
   and monochrome. Press each again to restore its previous setting.
1. Press `v`, Down, End, and `c`. Inspect escaped tabs and CR in the copy preview. The preview is
   display feedback; the example does not write to the system clipboard. Press `q` to quit.

## Reproduce Captures

```sh
cargo build --example viewer --locked
python3 scripts/capture.py examples/fixtures-unicode.tape
python3 scripts/capture.py examples/fixtures-unicode-text.tape
python3 scripts/capture.py examples/fixtures-scrolling.tape
python3 scripts/capture.py examples/fixtures-whitespace.tape
```

The tapes use keyboard input and output waits. PNG images are written under ignored `media/`; keep
them out of repository history. Unicode interaction captures use 730×528 pixels and text-only detail
captures use 730×350 pixels. Other captures use 1100×660 pixels, with 22-pixel padding and 22-pixel
JetBrains Mono. Compare paired images at the same size.

These are scenario and interaction-state comparisons, not baseline-main renderer improvements: main
cannot select the new fixtures, and this change does not alter rendering or event policy. The
fixture model test validates file/hunk counts and representative Unicode/line-ending inputs. Manual
steps above also cover interactions beyond the tape assertions; passing tapes alone does not
establish every manual observation.
