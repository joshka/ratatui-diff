# Pull Request Descriptions

A reviewer should learn the problem and result in ten seconds, then find enough evidence for a
closer read. Lead with a concrete use case and what changes for the caller or user. “Add
source-anchored selection” names an implementation; “Copying from a diff must recover source text
without gutters or wrap markers” explains its purpose.

## Shape for Skimming

- Open with the problem and result in one or two sentences.
- Put independent behavior changes in a few short bullets. Start each with the useful fact.
- Give evidence and validation distinct places. Use descriptive headings when they aid navigation.
- Keep image captions brief and adjacent to the image. Stack images when columns make text too
  small.
- Keep validation compact, retaining actual results, useful coverage, and material limitations.

Avoid both mandatory section templates and uninterrupted prose. Structure should reveal the change,
not decorate it. A small change may need only two paragraphs. Check the opening, headings, bullet
starts, and captions together: can a reader understand the change by skimming those alone?

## Choose What Earns Space

Explain new APIs through caller consequences: what the host supplies, what happens now or on the
next render, and which decisions remain with the host. Include compatibility changes. Link full
contracts rather than repeating them. For example, a host requests selection reveal after extending
focus; the PR need not repeat every caret-boundary rule.

Distinguish library behavior, example-host behavior, fixtures, and coverage. New fixture inputs do
not imply a renderer fix. A test-only PR explains the workflow and what its assertions establish.
Remove implementation inventories, shipping chronology, private workspace status, and repeated
summaries. Preserve reasons and qualifications needed to judge the change. Apply the claim and prose
review in [Documentation](documentation.md).

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
