---
title: Interactive viewer
description: Run the bundled example host and explore every feature from the keyboard and mouse.
---

[`examples/viewer.rs`](https://github.com/joshka/ratatui-diff/blob/main/examples/viewer.rs) is a
complete host application: terminal setup and restoration, an event loop, key bindings, mouse
hit-testing, search input, selection with a copy preview, and file and context folding. Every
screenshot on this site comes from it or from
[`examples/minimal.rs`](https://github.com/joshka/ratatui-diff/blob/main/examples/minimal.rs).

```sh
cargo run --example viewer
cargo run --example viewer -- --aardvark-ink
cargo run --example viewer --features syntax -- \
  --fixture syntax --aardvark-ink --syntax Coldark-Dark --syntax-contrast
```

![The viewer in unified view with its mode line above the diff and key hints below.](/ratatui-diff/assets/hit-testing.png)

## Keys

| Key                 | Action                                             |
| ------------------- | -------------------------------------------------- |
| `↑` `↓`, `j` `k`    | Scroll one row                                     |
| `PageUp` `PageDown` | Scroll one page                                    |
| `Home` `End`        | Start or end of the document                       |
| `←` `→`             | Scroll horizontally (unwrapped)                    |
| `[` `]`             | Previous or next hunk                              |
| `Tab` `Shift+Tab`   | Focus the next or previous visible file header     |
| `Enter`             | Collapse or expand the focused file                |
| `A` `Z`             | Expand or collapse all files                       |
| `e` `z`             | Expand the first visible context fold; fold all    |
| `d`                 | Toggle document totals                             |
| `s`                 | Toggle unified and split view                      |
| `w`                 | Toggle wrapping                                    |
| `t`                 | Toggle whitespace markers                          |
| `n`                 | Toggle line numbers                                |
| `i`                 | Toggle word highlights                             |
| `m`                 | Toggle monochrome                                  |
| `h`                 | Toggle syntax colors (`syntax` feature)            |
| `/`                 | Search; `Enter` to confirm, `Esc` to clear         |
| `f` `F`             | Next or previous match                             |
| `v`                 | Start a selection; arrows, `Home`, `End` extend it |
| `c`                 | Show the selected text's copy preview              |
| `p`                 | Probe a fixed cell's source coordinates            |
| `q`                 | Quit                                               |

Moving the mouse over source shows its file, side, line, and byte range in the status row; clicking
a file header toggles it.

## Options and fixtures

| Flag                | Effect                                                        |
| ------------------- | ------------------------------------------------------------- |
| `--aardvark-ink`    | Use the fixed RGB theme instead of the terminal palette       |
| `--sticky-headers`  | Keep the current file header visible                          |
| `--fixture NAME`    | Load another built-in fixture                                 |
| `--syntax THEME`    | Prepare syntax colors with a bundled theme (`syntax` feature) |
| `--syntax-contrast` | Adjust syntax colors for contrast against the diff theme      |
| `--syntax-licenses` | Print the bundled grammar and theme notices                   |
| `--capture VARIANT` | Render one widget-only frame for documentation captures       |

Fixtures: `showcase` (default), `context`, `files`, `alignment`, `unicode`, `unicode-text`,
`whitespace`, `multi-file`, `patch`, and `syntax`. `patch` parses `git show` output with a rename,
an edit, and a new file. All fixtures are built into the example; the viewer never reads files.

## How the screenshots are made

Each figure is captured with [Betamax](https://www.joshka.net/betamax/) from a tape in
[`examples/`](https://github.com/joshka/ratatui-diff/tree/main/examples) against the viewer built
from the same revision as the site. To regenerate them locally:

```sh
python3 scripts/site-media.py
cd site && pnpm install && pnpm dev
```
