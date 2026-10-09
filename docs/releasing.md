# Releasing

Release-plz prepares version/changelog PRs and publishes only after review and merge. CI checks the
exact revision before publication. Token-created PRs need explicit reusable-workflow CI because
GitHub suppresses ordinary PR triggers. The stable required check includes every blocking job.

Hosted setup requires a repository, main branch, a ruleset requiring `CI / required`, auto-merge
enabled, a protected `crates-io` environment, and a crates.io trusted publisher matching the release
workflow and environment. Enable private vulnerability reporting. These settings are not implied by
local configuration. No crates.io token is stored in this project.

Bootstrap README media in a dedicated `media-v1` GitHub release, separate from crate publication.
Run `just media`, review the captioned unified/split/whitespace PNGs and optional GIF, then upload
the ignored outputs as release assets. Only after assets exist should README embed their fixed
release URLs. Keep tapes and textual fixtures tracked.

Subsequent releases generate media in the same workflow using release-plz outputs, checkout the
released tag, and upload assets there. A failed capture leaves the published crate intact and
reports the missing assets; rerun the media workflow for repair. README continues to use its
reviewed fixed asset URLs until a deliberate update. Pin Betamax, theme, dimensions, and
deterministic inputs.

README uses static screenshots so readers control their pace. Capture at 1440×720 pixels with
22-pixel JetBrains Mono and Aardvark Ink. The optional GIF holds each view for five seconds and
hides transitions; captions explain the visible feature without requiring knowledge of the keys.
