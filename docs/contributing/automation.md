# Automation Policy

Accepted (issue #326): Renovate and the native bump loop are complementary,
not alternatives. Renovate proposes version bumps (scaffold plus weekly
grouped PRs); `dx bump`/`dx update` applies and verifies them locally with
resolver-owned refresh. Neither replaces the other.
Accepted: Renovate for dependency updates with update-only auto-merge under guardrails, no bot commits to `main` outside the merge path.
Delivered: native widen-one-requirement loop (issue #260) as the first-party apply/verify path; Renovate retained alongside it with the roles above.

## Native Bump Loop (delivered)

`dx bump <set:package> <version>` widens exactly one declared requirement
(`dx_bump` planning, `dx bump` dispatch), then `dx update <set>` refreshes
resolver-owned locks; Bazel and GitHub Actions verify file-only. The loop runs
one dep per PR, never batch: discover outdated (stable only, prerelease
follows upstream) → widen one → update → verify (regen evidence,
`preset.update --verify-only` flag-diff, `bazel build //...`,
`bazel test //...`, coverage/dogfood) → if green open one PR, if red discard
and record → reset to clean tree → next dep.

One dep per PR with an automerge on/off toggle only: when on, auto-merge
solely on the full required-check set; when off, leave PRs open. No grouping,
schedule, or dashboard knobs. The scheduled `bump.yml` runner uses
`GITHUB_TOKEN` with concurrency control so N open PRs do not stampede CI;
failures never retry-until-green; fork-safety and the human merge path below
preserved.

## Allowed

CI checks, a labeler, a changelog-presence check, the caller pin-sync
tests (delivered: `//tools/ci:examples_pins_test`,
`//tools/ci:consumer_pins_test`),
perf-baseline-bump PRs (open, tracked in the roadmap),
and Renovate version-bump PRs (allowed, weekly grouped).

Automation may open pull requests. Update-only auto-merge is allowed for
Renovate dependency-update PRs on green required checks (`bazel build //...`,
`bazel test //...`, corpus dogfood), patch/minor preferred. Otherwise it must
never push to `main`, commit on a contributor's behalf, or merge.

## Excluded

Dependabot-style version-bump bots are excluded by owner
decision (Renovate is the chosen updater).
Do not add per-automation exceptions without
reopening that decision.

## Merge Path

Bot-opened PRs go through the same human merge path as everything else:
reviewed, verified (`bazel build //...`, `bazel test //...`, corpus
dogfood), merged by hand. This upholds the release-hygiene gate
(release hygiene): no tags,
releases, or publication outputs without explicit owner approval.

## Scaffold

`dx init` ships `renovate.json` absent-only
(accepted): full manager
set from the start (`bazel` plus Cargo, npm/pnpm, GitHub Actions, Go),
grouped, scheduled weekly, reviewable PRs. Auto-merge off by default;
when enabled it is update-only as gated above. The repository's own
`renovate.json` is a snapshot of the scaffold held by
`//:renovate_parity_test` (schema plus byte snapshot, UPDATE_EXPECT
refreshes the golden).

## Hygiene Sweep

Pin single-sourcing plus stale-docs wording completed under issue #326:
Bazel pins track canonical `.bazelversion` and Bazelisk pins track canonical
`.github/actions/setup-bazelisk/action.yml` defaults (enforced by
`//tools/ci:pin_consistency_test`); caller-pin shape validation is symmetric
between `dx_ci::plan_pin_update` and the shell pin harnesses; Renovate versus
native is decided complementary above; sharding, Codecov, and sequential
wording tracks the as-built contract linked from each section.
