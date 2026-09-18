# Automation Policy

Decided: Renovate is the chosen updater (reopened decision; Renovate allowed).
Accepted: Renovate for dependency updates with update-only auto-merge under guardrails, no bot commits to `main` outside the merge path.

## Allowed

CI checks, a labeler, a changelog-presence check, the caller pin-sync
test (open work),
perf-baseline-bump PRs (open work),
and Renovate version-bump PRs (open work).

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
(automation backlog): full manager
set from the start (`bazel` plus Cargo, npm/pnpm, GitHub Actions, Go),
grouped, scheduled weekly, reviewable PRs. Auto-merge off by default;
when enabled it is update-only as gated above. The repository's own
`renovate.json` is held byte-identical to the scaffold by
`//:renovate_parity_test`.

## Hygiene Sweep

Pin single-sourcing plus stale-docs wording stays open under issue #326 (Bazelisk and
Bazel pins live in three places with no single source; caller-pin validation is
asymmetric; Renovate versus native updater is undecided; bare open-work placeholders
remain; sharding, Codecov, and sequential wording is stale versus as-built; single-source
the pins, decide the updater, polish placeholders, and fix stale claims).
