# ratatui-diff

<!-- rumdl-disable MD013 -->
<!-- cargo-rdme start -->

A navigable unified and split diff widget for Ratatui.

Prepare a [`DiffDocument`](https://docs.rs/ratatui-diff/latest/ratatui_diff/model/struct.DiffDocument.html) once, borrow it through [`Diff`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.Diff.html), and keep [`DiffState`](https://docs.rs/ratatui-diff/latest/ratatui_diff/widget/struct.DiffState.html)
between frames. The library owns comparison, patch parsing, layout, and navigation;
your application owns events, terminal setup, files, and repository access.

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState, ViewMode};

let document = DiffDocument::from_text("hello world\n", "hello Rust\n");
let widget = Diff::new(&document).mode(ViewMode::Split);
let area = Rect::new(0, 0, 80, 20);
let mut buffer = Buffer::empty(area);
let mut state = DiffState::new();
(&widget).render(area, &mut buffer, &mut state);
state.scroll_pages(1);
```

Input is UTF-8. Unified patches include only their available context; navigation
cannot recover omitted source lines. Combined merge diffs are unsupported.
Comparison and first layout are synchronous; prepare large documents outside the
event loop. Steady frames draw indexed visible rows without redoing comparison.
This pre-1.0 API may change between minor releases.

<!-- cargo-rdme end -->
<!-- rumdl-enable MD013 -->

## Views

Unified view keeps additions and deletions together, with changed words emphasized.

![Unified diff with line numbers and word highlights](https://github.com/joshka/ratatui-diff/releases/download/media-v1/unified.png)

Split view places old source on the left and new source on the right.

![Split diff with aligned old and new source](https://github.com/joshka/ratatui-diff/releases/download/media-v1/split.png)

[Whitespace markers](https://github.com/joshka/ratatui-diff/releases/download/media-v1/whitespace.png)
make spaces visible. The
[animated demo](https://github.com/joshka/ratatui-diff/releases/download/media-v1/viewer.gif) holds
each captioned view for five seconds.

## Development

See [Contributing](CONTRIBUTING.md), [architecture](docs/architecture.md),
[dependency decisions](docs/dependencies.md), and [deferred work](docs/roadmap.md). Run
`just example` for the interactive viewer. Generated Betamax media lives in GitHub release assets;
see [releasing](docs/releasing.md) for the initial media release setup.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
