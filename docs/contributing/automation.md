# Automation Policy

Decided per [issue #212](https://github.com/ralvik/rules_dx/issues/212) (reopened: Renovate allowed).
Accepted: Renovate for dependency updates with update-only auto-merge under guardrails, no bot commits to `main` outside the merge path.

## Allowed

CI checks, a labeler, a changelog-presence check, the caller pin-sync
test ([issue #209](https://github.com/ralvik/rules_dx/issues/209)),
perf-baseline-bump PRs ([issue #215](https://github.com/ralvik/rules_dx/issues/215)),
and Renovate version-bump PRs ([issue #3](https://github.com/ralvik/rules_dx/issues/3)).

Automation may open pull requests. Update-only auto-merge is allowed for
Renovate dependency-update PRs on green required checks (`bazel build //...`,
`bazel test //...`, corpus dogfood), patch/minor preferred. Otherwise it must
never push to `main`, commit on a contributor's behalf, or merge.

## Excluded

Dependabot-style version-bump bots are excluded by owner
decision (Renovate is the chosen updater; see [issue #212](https://github.com/ralvik/rules_dx/issues/212)).
Do not add per-automation exceptions without
reopening that issue.

## Merge Path

Bot-opened PRs go through the same human merge path as everything else:
reviewed, verified (`bazel build //...`, `bazel test //...`, corpus
dogfood), merged by hand. This upholds the release-hygiene gate
([issue #5](https://github.com/ralvik/rules_dx/issues/5)): no tags,
releases, or publication outputs without explicit owner approval.

## Scaffold

`dx init` ships `renovate.json` absent-only
([issue #3](https://github.com/ralvik/rules_dx/issues/3)): full manager
set from the start (`bazel` plus Cargo, npm/pnpm, GitHub Actions, Go),
grouped, scheduled weekly, reviewable PRs. Auto-merge off by default;
when enabled it is update-only as gated above. The repository's own
`renovate.json` is held byte-identical to the scaffold by
`//:renovate_parity_test`.
