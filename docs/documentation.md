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
link to it. Change-specific chronology belongs in PRs or jj descriptions.

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
