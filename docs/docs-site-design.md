# Documentation site proposal

Build a small static Astro/Starlight site at `https://www.joshka.net/ratatui-diff/`, with
`joshka.net/ratatui-diff` following the host's canonical-domain redirect. Help a caller choose
source retention, presentation, and interaction contracts before linking to complete Rustdoc. Reuse
Betamax’s GitHub Pages and Astro/Starlight setup. Page structure and layout below are illustrative;
the implementation priority is a reproducible subpath build and deployment.

This is a proposal, not an implemented site or deployment. No images are added in this planning
increment. A future ratatui.rs link requires separate work outside this repository.

## Inspected precedent

On October 9, 2026, inspection of the local Betamax `default` checkout's `site/astro.config.mjs`,
`package.json`, MDX pages, and `.github/workflows/docs.yml` established its Astro/Starlight setup,
explicit sidebar, MDX/GFM support, `site: 'https://www.joshka.net'`, and `base: '/betamax'`. Its
workflow hydrates LFS assets, checks/builds the site, verifies built PNG/GIF signatures, and
publishes `site/dist` with GitHub Pages. The Pages API reports workflow deployment and no project
CNAME. The [published landing page](https://www.joshka.net/betamax/) and
[themes guide](https://www.joshka.net/betamax/authoring/themes/) were inspected in the built-in
browser: search, theme selection, grouped navigation, on-page contents, code copy buttons, and
rendered tables work as the visible foundation.

Reuse that build and navigation structure with modest custom CSS. The tape-shaped hero belongs to
Betamax; ratatui-diff's landing page should devote that space to a readable diff and a small Rust
example. Correct the edit-link root to `site/src/content/docs/`: the inspected Betamax guide's
rendered edit link omits `site/`, although its landing page has an explicit correct link. Verify
rendered links rather than copying configuration unquestioningly.

## Initial page structure

| Page                        | Caller decision                                              | Canonical material                                                                                                           |
| --------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Overview                    | Does this widget fit my host?                                | README and library overview; one widget image, install command, quick-start link                                             |
| Render your first diff      | Generated text, patch, or structured input?                  | Rustdoc plus a runnable minimal example; state lifetime and render-before-navigation                                         |
| Choose a presentation       | Unified/split, wrapping, whitespace, theme, syntax?          | [Appearance](appearance.md) and [syntax](syntax-styling.md); matched option-to-image comparisons                             |
| Navigate and select         | How do events become source positions and exact copied text? | [Coordinates](interaction-coordinates.md), [selection](selection.md); host event example and paced interaction               |
| Retain, fold, and summarize | Which source must exist, and what can be hidden?             | [Folding](folding.md), statistics Rustdoc; full-retention versus patch limitations                                           |
| Contracts and limits        | What must the caller validate or schedule?                   | Links to Rustdoc, [architecture](architecture.md), [performance](performance.md), [roadmap](roadmap.md), and license notices |

Write the guide pages as caller narratives. Introduce a concrete source change, explain a choice and
its consequence, show the exact calls and output, then state boundaries and the next useful choice.
Keep maintainer release/testing guidance in repository docs. Do not copy all private architecture
into a user guide or recreate the API reference manually.

## Example content-to-output mapping

The site should help callers see the consequence of each configuration choice. A compact
presentation page can pair unified and split snippets with captures of the same settings fixture,
then show wrapping at a labeled narrow width. Put the changed option immediately beside its PNG;
explain blank alignment partners, continuation cues, and source-preserving selection. Add syntax
only with its feature flag, source choice, palette, and contrast policy stated explicitly.

Use the existing public calls in the compiled example:

```rust
use ratatui_diff::{Diff, DiffDocument, DiffState, DiffTheme, ViewMode};

let document = DiffDocument::from_text("timeout_ms = 500\n", "timeout_ms = 1500\n");
let mut state = DiffState::default();
let widget = Diff::new(&document)
    .theme(DiffTheme::aardvark_ink())
    .mode(ViewMode::Split)
    .wrap(true);
// In the host's draw callback:
// frame.render_stateful_widget(&widget, area, &mut state);
```

This illustrative block is not a newly validated capture fixture. The implementation should extract
it from a compiled example and capture that example’s output. Exact page layout is flexible; the
source-to-image correspondence is the contract. Label width, theme, and host setup, and offer static
checkpoints beside any optional paced interaction recording.

## Source of truth and capture provenance

Keep runnable Rust examples and original fixtures in `examples/`; keep deterministic Betamax tapes
there too. Extend the existing `--capture` and `--measure` viewer flows instead of implementing a
second renderer in JavaScript. Widget-only appearance captures exclude host chrome; interaction
captures show and explain the host's controls. Use PNG images to validate states and GIFs to
demonstrate transitions. Do not infer correctness from a recording alone.

Add a small capture manifest in the site implementation: page/figure ID, compiled example and code
region, fixture, widget options, tape, terminal cells and pixel size, theme, font, Betamax version
and digest, source revision, output path, and checksum. Generate code includes and figure metadata
from it. A manifest assertion should reject missing or duplicated figure IDs, absent code regions,
and mismatched image references. Keep source revision separate from output checksums to avoid
self-referential commits. Existing captures can seed the gallery after checking provenance; do not
claim that unrelated viewer screenshots are output of a minimal snippet.

Repository guides own contracts today. During site implementation, extract shared prose or move its
canonical owner explicitly and leave links behind. Avoid independently maintained copies of syntax
limits, source-retention rules, and performance numbers. Rustdoc remains the detailed API authority.

## Subpath and deployment

Configure `site: 'https://www.joshka.net'`, `base: '/ratatui-diff'`, and `trailingSlash: 'always'`.
Astro's [base option](https://docs.astro.build/en/reference/configuration-reference/#base) sets the
path prefix; construct custom links and public-media URLs with `import.meta.env.BASE_URL` instead of
root-relative `/assets/...`. Use Starlight slugs for sidebar links and check the generated canonical
URLs. Test both `/ratatui-diff` and `/ratatui-diff/`, preserving query strings and fragments through
any canonical redirect.

Propose GitHub Pages workflow deployment following the verified Betamax model. Check/build on PRs;
upload a preview artifact without deploying it. Deploy only a successful main build through a
`github-pages` environment with `pages: write` and `id-token: write` on the deployment job. Pin
actions and Node/pnpm tools; commit the lockfile. Add the docs build to `CI / required` when the
site exists, with filters that also include referenced Rust examples, fixtures, tapes, and shared
prose.

Before deployment, verify this repository's Pages source, permissions, environment, and inherited
user-site domain routing. Those settings were not verified for ratatui-diff. Do not add a project
CNAME for the existing shared domain without checking the user-site topology. Confirm HTTPS,
canonical host behavior, nested direct loads, search index URLs, 404 handling, and real served media
bytes after deployment. Site configuration alone does not mount a directory on an unrelated host.

## Media storage and validation

The user's explicit instruction authorizes Git LFS for committed documentation-site imagery,
superseding the older blanket “generated images stay untracked” rule for curated site assets only.
Keep intermediates ignored. Track narrowly scoped paths such as `site/public/assets/diff/*.png` and
`site/public/assets/diff/*.gif`; avoid repository-wide extension rules. README release media retains
its existing release-asset workflow.

Before publication, verify attributes and the actual exported Git blob for every asset: jj must not
be assumed to apply LFS clean filters. The blob must contain the LFS version, SHA-256 OID, and size,
rather than PNG/GIF bytes. Confirm LFS object upload and successful fresh hydration. Keep this
verification with the publication coordinator; use a narrow Git LFS handoff if exported jj blobs
contain raw imagery instead of pointers.

The future docs build must use `actions/checkout` with `lfs: true`, check hydrated source assets,
then check built assets for PNG/GIF signatures and reject pointer text. Verify served signatures
after deployment as well. GitHub displaying a pointer correctly is insufficient evidence that Pages
contains real pixels.

For each update, rerun its compiled example and tape on an identified revision, retain baseline and
candidate separately, inspect labeled matched PNG images, and run focused cell/style/source
assertions. Review paced GIF captions and timing separately. Validate the manifest and links,
compile extracted snippets, run the site check/build, and inspect narrow/wide pages with keyboard
navigation. Record actual evidence and unresolved cases in the PR; changed pixels alone do not
justify updating tests.

## Accessibility and acceptance

Retain Starlight's skip link, semantic headings, searchable navigation, focus indicators, and theme
selection. Keep plus/minus markers, labels, and visible whitespace cues so meaning does not depend
on hue. Give images concise alternative text describing the option and visible result; include
selectable source and a caption for details. Label hexadecimal values with color swatches.

Give each image intrinsic dimensions and a readable responsive layout. Do not autoplay animation as
the only explanation: provide a static poster and user-controlled reveal/play, honor reduced motion,
and avoid downloading GIFs until requested. Check code-copy controls, horizontal overflow, zoom,
light/dark chrome, and keyboard access. Theme screenshots retain their own explicit capture palette
regardless of site theme.

The first implementation is ready for review when the Pages/Starlight build works, guide prose
explains caller choices, every figure maps to compiled source and a deterministic capture, all media
is hydrated and validated, and a preview works under the exact subpath. Deployment and the later
ratatui.rs link remain separate approvals and work units.
