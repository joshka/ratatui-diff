# Develop a TUI feature with a visible feedback loop

Scrolling a long diff used to remove the file path and change counts. This worked example adds
opt-in sticky file headers to `ratatui-diff`, using one Betamax scenario for development feedback,
terminal acceptance, review media, and documentation. The application runs in a real PTY. Its
screenshots are captured output, not mockups.

The reusable workflow is below; [scenario details](#ratatui-diff-scenario) describe this library's
API and fixture. Cross-repository placement in Betamax's documentation is coordinated by the
ratatui-diff planning task. This document is the handoff, not a claim that Betamax's site is
updated.

## Result and caller configuration

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;
use ratatui_diff::{Diff, DiffDocument, DiffState};

let document = DiffDocument::from_text("old\n", "new\n");
let mut state = DiffState::new();
let area = Rect::new(0, 0, 80, 20);
let mut buffer = Buffer::empty(area);
let diff = Diff::new(&document).sticky_file_headers(true);
(&diff).render(area, &mut buffer, &mut state);
```

In an application, retain `DiffState` across frames and render `&diff` with
`frame.render_stateful_widget`. Sticky headers are disabled by default. Enabling them reserves one
row for the current file, below optional `.show_stats(true)` document totals. Natural topmost
headers are not duplicated. The header switches at the next file's scroll position and keeps its
existing `HitTest::FileHeader` folding control. One-row content viewports use ordinary scrolling.
Paths use the existing ellipsis rules; extremely narrow areas cannot display both a path and
complete counts.

Before, after eight down-scrolls with the default configuration:

![Before: scrolling loses the file path and totals](assets/sticky-headers/before-scrolled.png)

With `.sticky_file_headers(true)`, the same scroll leaves the header above the source:

![After: the path and colored counts remain visible](assets/sticky-headers/after-scrolled.png)

With `.mode(ratatui_diff::ViewMode::Split).sticky_file_headers(true)`, one header spans both panes:

![After: split panes share the current file header](assets/sticky-headers/after-split.png)

The captioned comparison replays the same start, eight-row scroll, and split-view transition against
both executables. Pauses at checkpoints give readers time to compare:

![Before and after: one scenario, two executable revisions](assets/sticky-headers/comparison.gif)

## Reusable workflow

### Capture the baseline before changing behavior

1. Investigate the application and public API. Confirm a concrete visible problem and agree on
   behavior before editing the implementation.
1. Build the starting revision. Preserve the executable outside the disposable workspace, together
   with the revision, build log, tool version, and checksums.
1. Run the application through Betamax with an explicit fixture, terminal geometry, theme, and font.
   Save PNG checkpoints, screen/scrollback JSON, and a short GIF.
1. Inspect the actual images and structured state. Show labeled images and the GIF in live progress
   commentary so the human can assess the starting point.

For this run, the baseline was captured before editing Rust source. After eight scrolls,
`AssertAbsent "src/routes.txt"` passed while `route 24: revised` remained visible. That established
why keeping the header would be useful. The baseline JSON had zero scrollback rows: this viewer uses
the alternate screen, so scrollback does not preserve the scrolled-off file identity.

### Implement, replay, inspect, correct

After meaningful changes, run the fast scenario again. Use waits and focused cell/style assertions
to establish observable states. Save each iteration to a fresh directory. A failed assertion is
useful evidence: retain its actual PNG, JSON, cell dump, and diagnostic text before correcting code
or an incorrect expectation.

The first implementation replay here failed before drawing the diff. The new `--sticky-headers` flag
reached the render path but was still rejected by the existing argument matcher. The preserved
failure PNG showed the usage error, and the screen JSON confirmed that the application had exited.
The correction consumes the flag before matching remaining arguments. This was an application
integration failure, not a sticky-header rendering failure.

Two initial Rust assertions also assumed that a split left-pane hit contained the new source side.
They were corrected to assert the old side. The tape authoring pass then caught `Esc`, which is not
Betamax syntax; the supported command is `Escape`. These failures were not manufactured.

The next successful replay was inspected in unified, split, and narrow views and shown in live
commentary. The path retained its cyan header style, counts kept their green/red styles, and the
first scrolled source row remained visible below the header. Passing assertions alone would not have
established that the layout looked acceptable.

### Separate acceptance from visual approval

The canonical [tape](../examples/terminal-ux-sticky.tape) uses 250 ms settling pauses and Betamax's
settled assertions. Relevant commands include:

```text
Type "jjjjjjjj"
Sleep 250ms
AssertText "route 24: revised"
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
AssertStyle 21 3 2 '{"fg":"#75cf84","bg":"#0f141f","bold":true}'
Screenshot media/terminal-ux/sticky/scrolled.png
State media/terminal-ux/sticky/scrolled.json
```

Coordinates are zero-based terminal cells, excluding image padding and captions. The style assertion
checks the addition count without snapshotting unrelated footer text. The tape also asserts the
removal count and header path styles.

[JSON comparison](../scripts/compare-sticky-json.py) compares the same named checkpoints. It
normalizes styled spans to effective styles rather than comparing palette indexes. At `scrolled`,
`split`, and `narrow`, it verifies that the baseline source rows reappear one row lower with the
same text and styles. It separately checks file identity, folding, totals, the final line, matching
geometry, and empty scrollback. These are public behavior checks, not pixel-perfect approval.

Public Rust tests exercise hit-testing, both source sides, file navigation, context expansion,
source reveal, narrow wrapped layouts, end clamping, tiny areas, and document replacement through
the exported API. They complement the terminal session; they do not substitute for it.

### Curate the review recording from the same scenario

The [small adapter](../scripts/sticky-headers.py) derives baseline expectations and paced capture
from the checked-in tape. It changes artifact destinations, baseline-only expectations, captions,
and checkpoint pauses. It does not carry an independent scrolling sequence. The baseline uses
`Enter` to fold the current file; the candidate clicks the newly pinned header. This intentional
input difference tests the new control, while the scroll and resize sequences remain identical.

The comparison GIF uses the first three verified checkpoints, replayed before and after, with
2.5-second holds. Full acceptance covers the remaining checkpoints. Images extracted from the
encoded GIF were inspected too. The first composition attempt repeated `Set Shell` after runtime
commands and was rejected; the correction keeps setup only at the beginning. Inspection of the next
encoded GIF showed that slowing every sleep left old captions over transitions too long, so only
checkpoint holds were lengthened. All attempts remain in the evidence archive.

Betamax 0.1.22 does not provide named scenario/checkpoint variants in this tape. The adapter is
specific to this example. A reusable include/profile facility is a possible Betamax follow-up; this
proof of concept does not add a general scenario framework.

### Publish durable evidence and hydrate documentation assets

The PR embeds matched PNGs and the comparison GIF. The evidence archive retains tapes, JSON, logs,
real failure output, tool/source identity, frozen executables, and checksums. Local raw evidence
lives outside `work/`, under the main checkout's ignored `media/sticky-headers/` directory.

Only seven curated PNG/GIF files are committed under `docs/assets/sticky-headers/`, with narrowly
scoped Git LFS rules. Do not assume jj applies Git clean filters: inspect exported Git blobs and
upload the corresponding LFS objects before publication. The documentation job checks out with
`lfs: true` and runs [the hydration guard](../scripts/check-sticky-media.py), which verifies image
magic and the SHA-256 values in `media.json`. A successful Markdown or Rustdoc build alone would not
prove that image bytes were hydrated.

When moving this walkthrough into Betamax's site, preserve the LFS rules and run the same guard
against copied build-output media as well as checkout files. That site integration is a separate
handoff; this repository has no site build for these Markdown guides.

## Ratatui-diff scenario

### Inputs and provenance

| Input                        | Recorded value                                                           |
| ---------------------------- | ------------------------------------------------------------------------ |
| Starting source              | `98707e6d46852cdc3dca10f8dc4187c2d80b2d1f`                               |
| Feature capture source       | `1150c93dbdfa1cac9939a448c940482f42d0cc50`                               |
| Application                  | `examples/viewer.rs`, built with `cargo build --example viewer --locked` |
| Fixture                      | `multi-file`, generated by `examples/viewer_fixtures/mod.rs`             |
| Terminal                     | 92×25 cells; resize checkpoints at 50×16, 50×7, and 50×5                 |
| Image settings               | 1340×680 canvas, JetBrains Mono 24, letter spacing 0, padding 20         |
| Theme                        | Aardvark Ink for both terminal and widget                                |
| Betamax                      | Released 0.1.22, source `5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2`       |
| macOS arm64 archive SHA-256  | `0fe9e07cf963a668725508fd5afa62120245950fded75718898546c4de22f98e`       |
| Linux x86-64 archive SHA-256 | `497c0bbf6da724e90be49e48ade317d99567e4b03e4e86f03c0dac691ec6358f`       |

Each replay's `manifest.json` records absolute executable paths and executable/tape/fixture SHA-256
values. Source snapshots accompanying the evidence preserve intermediate revisions even if the PR is
subsequently squashed. The feature capture revision identifies the Rust implementation; later
changes to documentation, CI, and capture pacing do not change that executable.

### Task prompt

The task was to investigate and implement sticky file headers, then demonstrate the real
agent-driven development lifecycle: baseline capture before edits; labeled PNG/GIF progress;
deterministic terminal assertions and JSON inspection; retained real failures; matched, paced review
media; required CI; a reviewable PR; durable provenance; LFS-backed documentation images; and a
self-contained Betamax docs handoff. The implementation had to fit the backend-independent library,
use an isolated jj workspace under `work/`, and keep cross-repository docs coordination with the
parent task. The original full task prompt is preserved with the evidence.

### Checkpoints

| Identity    | Interaction and contract                                            |
| ----------- | ------------------------------------------------------------------- |
| `start`     | Natural file header, no duplicate                                   |
| `scrolled`  | Eight down-scrolls; path and styled counts pinned only in candidate |
| `split`     | Switch to split mode at the same source anchor                      |
| `narrow`    | Wrap and resize to 50×16; retain identity and source content        |
| `boundary`  | Search for `setting 24`, clear query; current file becomes settings |
| `folded`    | Candidate header click / baseline Enter folds settings              |
| `totals`    | Fold all, enable totals, Home; preserve supplied-line counts        |
| `one-row`   | 50×7 leaves one widget row for document totals                      |
| `zero-area` | 50×5 leaves no diff area; totals disappear                          |
| `end`       | Restore 92×25, expand all, disable totals, End reaches `check 47`   |

The fixture retains separate hunks with omitted context; those gaps cannot be expanded. Retained
context folding is covered in the public Rust test with a complete-source fixture. This distinction
avoids claiming that a tape can recover unavailable patch context.

### Reproduction commands

Use the PR checkout for scripts and the released Betamax binary. Preserve baseline and candidate
executables in an evidence directory outside the workspace. This example uses `$EVIDENCE` for that
absolute directory and `$BETAMAX` for the verified tool executable.

```sh
jj workspace add work/sticky-baseline -r 98707e6d46852cdc3dca10f8dc4187c2d80b2d1f
cargo build --manifest-path work/sticky-baseline/Cargo.toml --example viewer --locked
mkdir -p "$EVIDENCE/baseline" "$EVIDENCE/candidate"
cp work/sticky-baseline/target/debug/examples/viewer "$EVIDENCE/baseline/viewer"
cargo build --example viewer --locked
cp target/debug/examples/viewer "$EVIDENCE/candidate/viewer"
revision=$(jj log -r @ --no-graph -T commit_id)
python3 scripts/sticky-headers.py --viewer "$EVIDENCE/baseline/viewer" \
  --revision 98707e6d46852cdc3dca10f8dc4187c2d80b2d1f \
  --baseline --output "$EVIDENCE/before"
python3 scripts/sticky-headers.py --viewer "$EVIDENCE/candidate/viewer" \
  --revision "$revision" --output "$EVIDENCE/after"
python3 scripts/compare-sticky-json.py "$EVIDENCE/before" "$EVIDENCE/after"
python3 scripts/sticky-headers.py --viewer "$EVIDENCE/candidate/viewer" \
  --revision "$revision" --paced --comparison-baseline "$EVIDENCE/baseline/viewer" \
  --output "$EVIDENCE/review"
```

The adapter refuses to overwrite an existing manifest. Use a new output directory for each
iteration. `BETAMAX_FAILURE_DIR` is set per run; an unsuccessful replay returns nonzero while
retaining diagnostics. To run the unmodified acceptance tape directly:

```sh
export RATATUI_DIFF_VIEWER="$EVIDENCE/candidate/viewer"
export BETAMAX_FAILURE_DIR="$EVIDENCE/direct-failures"
"$BETAMAX" run examples/terminal-ux-sticky.tape
```

### Required CI and library integration

The existing `terminal-acceptance` job runs the tape through
`joshka/betamax-action@4f9e3b05a390d0bb16e72af92e3995541a532c79`, explicitly selecting release
0.1.22 and the Linux archive checksum above. It produces a review gallery and remains a dependency
of the `required` aggregate. The existing always-run artifact upload retains PNG/JSON checkpoints
and diagnostics. Expiring Actions artifacts supplement the durable review archive.

The released `betamax-core` source was inspected before choosing integration. It exports `Tape`,
`Runner`, `RunOptions`, `RunArtifacts`, and `TerminalSession`. `Runner::run_artifacts` can return
structured final state, and the tape supports `AssertState` as well as focused assertions. A public
`state_diff` module exists. This example exercises the released CLI and JSON comparison; it does not
execute a direct Rust embedding of Betamax.

No dedicated Insta adapter was found in the inspected release manifests. A concrete follow-up for
Betamax is a compiled integration example using `Runner::run_artifacts` and a deliberately selected,
stable state projection with `insta::assert_json_snapshot!`, documenting native dependencies,
platform support, normalization, and update review. JSON being serializable is not evidence that
such an integration has been built or tested.

### Validation and limitations

The focused public-API tests and both terminal acceptance variants passed, and the eight-checkpoint
JSON comparison passed. PNGs from unified, split, and narrow states and frames from the encoded GIF
were visually inspected. Full repository and hosted validation results are recorded in the PR and
evidence logs so this document does not conflate an intermediate capture with a later CI run.

The terminal scenario uses ASCII paths/source, one theme/font, and deterministic in-memory data.
Unicode, binary/metadata files, and other themes retain the existing broader suite's coverage but
are not all demonstrated in this sticky-header tape. There is no OS clipboard claim or performance
benchmark claim. Semantic equality does not guarantee attractive appearance, and attractive
screenshots do not establish correct source mappings.
