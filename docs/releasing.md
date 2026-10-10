# Releasing

Release-plz prepares version/changelog PRs and publishes after review and merge. The release job
first calls reusable CI for the main revision, then uses the `crates-io` environment and OIDC. No
crates.io token is stored in project configuration. Hosted prerequisites must be checked separately
before authorizing publication.

## Required evidence

The `CI / required` aggregate fails unless formatting, platform/MSRV tests, stable/beta Clippy,
documentation/package checks, policy checks, semver checks, and terminal acceptance all succeed.
Tests include all features, doc tests, and default-disabled builds. Terminal acceptance downloads
Betamax 0.1.22 with a verified digest, runs the viewer scenarios, rebuilds with `syntax`, and runs
its interaction tape. Failure diagnostics upload with `always()`. See [testing](testing.md).

Release PRs created with `GITHUB_TOKEN` do not trigger ordinary pull-request workflows. The release
workflow explicitly dispatches `ci.yml` on the returned PR branch. Verify the successful check
belongs to the current PR head before merging; a dispatch request alone is not passing evidence. Run
`just check`, `just msrv`, and package validation as described in the testing guide, and record
which checks CI covers and which are pending. Performance measurements are evidence with scoped
fixtures, not absolute shared-runner release gates; see [performance](performance.md).

Syntax is optional but distributable assets add obligations: retain `LICENSE-syntax-assets.txt`,
check default-disabled and enabled packages, and review the documented native Oniguruma build
requirement and narrow dependency-audit exception. See [syntax styling](syntax-styling.md).

## Hosted readiness audit

Read-only GitHub API inspection on October 9, 2026 found the following. Recheck before release;
these observations are not permanent guarantees or authorization to change settings.

| Prerequisite                      | Verified state                                                                  | Required follow-up                                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Public repository and main branch | `joshka/ratatui-diff`, default branch `main`                                    | Confirm release revision and version                                                                                        |
| Required check protection         | Rulesets API returned `[]`; main protection API returned “Branch not protected” | Require `CI / required` through a main ruleset or protection                                                                |
| Auto-merge                        | Repository API reports `allow_auto_merge: false`                                | Enable if using the planned auto-merge flow                                                                                 |
| `crates-io` environment           | Exists; no protection rules or deployment branch policy; admin bypass enabled   | Configure reviewed protection and branch restrictions                                                                       |
| Trusted publisher                 | Workflow has `id-token: write` and environment `crates-io`                      | Verify crates.io owner-side publisher matches repository, `release-plz.yml`, and environment; not verified by GitHub config |
| Private vulnerability reporting   | API reports enabled                                                             | Keep enabled                                                                                                                |
| Published baseline                | Registry script returned `0.0.0`                                                | Inspect the published crate and intended first substantive release; do not treat this as an absent baseline                 |
| README media bootstrap            | `media-v1` release includes unified/split/whitespace PNG images and viewer GIF  | Verify displayed URLs and images before changing README                                                                     |

The repository workflow and media release are configured, but branch and environment protection
remain pending. This audit did not publish a crate, change hosted settings, or test trusted
publishing credentials. An authenticated crates.io owner must verify the publisher configuration.

## Release media

README uses reviewed static screenshots so readers control their pace. Bootstrap media lives in the
separate `media-v1` release. Run `just media`, inspect the captioned unified/split/whitespace PNG
images and optional GIF, then upload ignored outputs as release assets. Update README's fixed
release URLs only after assets exist. Keep tapes and original textual fixtures tracked.

Subsequent releases call the media workflow with release-plz's tag, check out that tag, generate
media, and upload it there. Capture failure leaves a published crate intact; repair by rerunning the
media workflow for the existing tag. Pin Betamax, theme, dimensions, and deterministic inputs. The
README capture uses 1440×720 pixels, 22-pixel JetBrains Mono, and Aardvark Ink. The optional GIF
holds each view for five seconds and hides transitions; captions explain features independently of
key bindings.

Curated documentation-site images follow the separately authorized Git LFS policy in
[the site proposal](docs-site-design.md#media-storage-and-validation). Ordinary capture outputs and
review intermediates stay ignored. Site publication remains a separate implementation and deployment
step; a future ratatui.rs link is outside this repository.
