# Documentation

Write for a reader arriving without chat history. Lead with the result or behavior, then provide the
evidence and limitations needed to assess it. Status reports identify the subject, completed work,
actual checks, and remaining work. Prefer the shortest explanation that preserves meaning.

Preserve knowledge whose absence would force material reinvestigation: invariants, ordering,
surprising constraints, failure modes, measured tradeoffs, and plausible but incorrect
simplifications. Clearer code comes first; comments should explain durable knowledge rather than
translate syntax. Reusable explanations reduce repeated investigation by people and future agents.

Put caller contracts in Rustdoc, local invariants beside code, cross-module explanations in
architecture docs, and dependency choices in their decision document. Keep one canonical owner and
link to it. Change-specific context belongs in PRs or jj descriptions. Follow
[Pull Request Descriptions](pr-writing.md) for review prose.

Ground rationale in code, tests, history, measurements, or attributed decisions before writing it.
Current code establishes behavior but may not establish intent. Distinguish verified facts,
preferences, plans, and unresolved inference. Never invent intent or precise numbers to improve
prose.

Review substantive drafts twice: first for structure and claims, then for generated prose tells.
Remove repeated framing, summaries, vague praise, inflated stakes, stock AI wording, forced
contrasts, rhetorical questions, and decorative lists. Repair weak sentences before deleting useful
reasons, conditions, qualifications, and examples. These are editing signals, not proof of
authorship.

Own AI-assisted prose: read it, verify its claims, and curate it into an explanation you can defend.
Do not publish raw exploration or assume private context is shared. Stop reviewing when further
changes only exchange equivalent wording; complete correctness and validation gates first.

Run `just fmt-md` and `just fmt-md-check`. Prose wraps at 100 columns; tables are aligned and may be
wider. Keep these guides self-contained rather than requiring a private preference checkout.

## Writing for Users

Review each paragraph for the question it answers and the action or understanding it enables. README
and usage docs should help callers choose inputs, render a diff, configure it, and navigate. Keep
design alternatives and implementation history in architecture or decision docs. A technically
accurate paragraph can still belong on a different page.

Lead with concrete behavior. Avoid page narration ("this section explains"), teaching-order filler
("let's break this down"), unearned praise ("straightforward"), and forced contrasts. Name the type,
input, effect, or limitation instead. Use consistent terms: do not alternate between "word
highlights" and "word emphasis" merely for variety. Preserve ordinary warmth and sentence variety.

Let examples show syntax. Surrounding prose should explain prerequisites, consequences, or choices
that readers cannot infer from the calls. For example:

- Before: "Render a shared reference with `render_stateful_widget`."
- After: "Keep `DiffState` between frames to retain the viewport and cached layout."

Distinguish product behavior from the demonstration's setup. An in-memory backend makes an example
runnable; it is not a requirement for using the widget. State what transfers to an application and
link to the runnable viewer when terminal setup or event handling is needed.

Check neighboring paragraphs for repeated ownership descriptions and lifecycle advice. Give each
fact a useful home; repeat it only where a reader entering at that point needs it. Do not remove
render-before-navigation requirements or cache lifetime rules just because they mention internals.
Those details affect caller behavior.

Review the result for meaning as well as tone. Preserve conditions, failure cases, useful examples,
and justified limitations. A phrase list can flag candidates, but it cannot decide which explanation
a reader needs. Stop when another pass would only substitute equivalent wording.

Show a color swatch beside hexadecimal color values in visual documentation. Keep the value readable
and selectable; the swatch supplements the label.
