# Documentation site

The user guide at <https://www.joshka.net/ratatui-diff/> is an Astro/Starlight project in `site/`.
It explains caller choices with captured output and links to Rustdoc for exact contracts. Repository
guides in `docs/` stay the maintainer-facing source for design, testing, and measurement detail;
summarize them on the site and link back rather than copying contracts.

## Build locally

```sh
just site       # capture screenshots, run site snippets, check and build site/dist
just site-dev   # live-reload server at http://localhost:4321/ratatui-diff/
```

`just site` needs Betamax 0.1.22, JetBrains Mono, Node 24, and pnpm 11 (the `packageManager` pin in
`site/package.json`). Run `just site-media` once before `just site-dev` so pages have images.

## Hosting

`.github/workflows/docs.yml` builds on pull requests that touch the site or its inputs, and builds
and deploys every push to `main` through the `github-pages` environment. The repository's Pages
source must be **GitHub Actions**. No project CNAME is needed: the `joshka.github.io` user site owns
the `www.joshka.net` custom domain, so project sites are served beneath it at `/ratatui-diff/`.

`astro.config.mjs` sets `site: 'https://www.joshka.net'`, `base: '/ratatui-diff'`, and
`trailingSlash: 'always'`, and redirects a bare `/ratatui-diff` to the canonical trailing-slash URL.
Starlight does not prefix the base onto Markdown URLs, so page images use absolute
`/ratatui-diff/assets/...` paths and internal links are relative to the page's trailing-slash route.

## Screenshots

Generated images are never committed. `scripts/site-media.py` builds the viewer and minimal examples
(with the `syntax` feature), replays the tapes listed in its `TAPES`, and copies the outputs named
in `FIGURES` into ignored `site/public/assets/`. Each figure therefore matches the code at the built
revision. Sticky-header comparisons are reviewed evidence from an earlier revision pair and are read
from Git LFS; the script smudges their pointers directly into the output so a jj working copy never
snapshots raw image bytes over a pointer.

To add a figure, write or reuse a tape in `examples/`, add it to `TAPES` and its output to
`FIGURES`, and reference `/ratatui-diff/assets/<name>`. Describe the visible option and result in
the alt text. `site-media.py --check site/dist` verifies that every figure in the built site has
PNG/GIF bytes.

## Code examples

`scripts/check-site-snippets.py` compiles and runs every `rust` block on the site against this
crate. A block without its own `use ratatui_diff` continues the page's first block that has one. The
getting-started page embeds `examples/minimal.rs` directly, so its full program is built by Cargo.

## Writing pages

Follow [documentation](documentation.md): lead with what the caller chooses, show the call and its
captured result side by side, then state limits. Keep prose wrapped at 100 columns; `rumdl` ignores
link-target and inline-HTML checks for `site/` because links point at built routes and figures use
MDX components.
