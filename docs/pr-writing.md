# Pull Request Descriptions

A reviewer should learn the problem and result in ten seconds, then find enough evidence for a
closer read. Lead with a concrete use case and what changes for the caller or user. “Add
source-anchored selection” names an implementation; “Copying from a diff must recover source text
without gutters or wrap markers” explains its purpose.

## Shape for Skimming

- Open with the caller need and the newly available result in one or two sentences.
- For new capabilities, show the enabling public calls near the top. A short Rust example often
  makes the feature usable without opening another page; links supply the full contract.
- Put independent behavior changes in a few short bullets. Start each with the useful fact.
- Give evidence and validation distinct places. Use descriptive headings when they aid navigation.
- Keep image captions brief and adjacent to the image. Stack images when columns make text too
  small.
- Keep validation compact, retaining actual results, useful coverage, and material limitations.

Fit the description to the change. A capability PR usually needs the caller result, essential usage,
defaults and compatibility, then supporting evidence and compact validation. A bug fix can describe
its trigger and corrected result; documentation and maintenance changes can explain the reader or
maintainer benefit. Omit usage examples and sections that add no useful information. A small change
may need only two paragraphs. Check the opening, headings, bullet starts, and captions together: can
a reader understand the change by skimming those alone?

## Choose What Earns Space

Explain what library callers can now do and name the exported APIs that enable it. State whether the
behavior applies by default or requires a setter, feature flag, or other opt-in. Describe what the
host supplies, when the effect becomes visible, and which decisions remain with the host. Include
material compatibility changes, such as a new enum variant affecting exhaustive matches.

Show a concrete Rust example when it helps callers adopt the capability. Include imports and enough
setup to make the operation clear; omit terminal scaffolding when it is unrelated. Keep the example
before screenshots and validation, and link canonical Rustdoc or a usage guide for the remaining
rules. Verify the calls against exports and public tests. A private helper or prototype does not
establish public availability. Label proposals and unreleased behavior accurately; distinguish what
the PR makes available from what is already in a published release.

Distinguish library behavior, example-host behavior, fixtures, and coverage. New fixture inputs do
not imply a renderer fix. A test-only PR explains the workflow and what its assertions establish.
Remove implementation inventories, shipping chronology, private workspace status, and repeated
summaries. Preserve reasons and qualifications needed to judge the change. Apply the claim and prose
review in [Documentation](documentation.md).

### Example: Apply Saved File Expansion

The [file-folding PR](https://github.com/joshka/ratatui-diff/pull/14) explains the compact overview
and links its contracts, but its opening does not show the calls needed to prepare that overview. A
usable introduction can keep the same caller benefit and add the essential operation.

Before:

> Whole-file folding lets a host show a compact multi-file overview without discarding source or
> nested context preferences. Hosts can apply saved expansion before the first frame.

After:

> Hosts can restore saved file expansion before the first frame, showing a compact overview while
> retaining source text and nested context preferences. Get a file's handle from the document and
> apply its saved preference to the retained state before rendering:

```rust
use ratatui_diff::{DiffDocument, DiffState};

let document = DiffDocument::from_text("old\n", "new\n");
let mut state = DiffState::new();
let file = document.file_fold(0).unwrap();
assert!(state.set_file_expanded(&file, false));
```

> Files default to expanded; this request retains the header and hides the file's content on the
> next render. The host owns persistence and input handling. Handles belong to their document;
> rendering a replacement document resets file preferences. Exhaustive `HitTest` matches must handle
> the new `FileHeader` variant. See the [folding guide](folding.md) for the full contract.

The snippet uses the public API exercised by `tests/programmatic_folding.rs` and the `file_fold`
Rustdoc example. In a feature PR, follow this introduction with relevant visual evidence and a short
validation report.

## Ground Claims and Evidence

Read the final diff, relevant tests, and introducing or corrective history before drafting. Current
code establishes behavior; history or an attributed decision may establish rationale. Summarize a
preceding PR's relevant contract beside its link so readers need no chat history.

Follow the [visual evidence workflow](testing.md#visual-evidence-during-development-and-review).
Preserve useful durable URLs and state matched settings once per comparison. Caption what to observe
and intentional differences. Label existing behavior and fixture states as scenarios; images cannot
prove pre-redraw coordinate invalidation or exact copied bytes. Inspect GitHub rendering when
practical.

Distinguish local checks from hosted results and historical evidence from checks on the final
revision. Keep meaningful benchmark conditions or link their durable record. Rewriting a merged body
does not rerun implementation validation. Replace future-tense CI notes once results are known.

GitHub paragraphs must not be hard-wrapped. Use structured API input or `gh --body-file` for
multiline Markdown. Review once for context, skimming, and claim accuracy, then for unnecessary
detail and generated prose tells. Brevity should reduce reader effort without removing necessary
evidence.
