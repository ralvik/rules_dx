# Automation Policy

Decided per [issue #212](https://github.com/ralvik/rules_dx/issues/212).
Accepted: no version-bump bots, no auto-merge, no bot commits to `main`.

## Allowed

CI checks, a labeler, a changelog-presence check, the caller pin-sync
test ([issue #209](https://github.com/ralvik/rules_dx/issues/209)), and
perf-baseline-bump PRs ([issue #215](https://github.com/ralvik/rules_dx/issues/215)).

Automation may open pull requests. It must never push to `main`,
commit on a contributor's behalf, or merge.

## Excluded

Renovate and Dependabot-style version-bump bots are excluded by owner
decision; this stays decided (see [issue #212](https://github.com/ralvik/rules_dx/issues/212)).
Do not reintroduce `renovate.json` or per-automation exceptions without
reopening that issue.

## Merge Path

Bot-opened PRs go through the same human merge path as everything else:
reviewed, verified (`bazel build //...`, `bazel test //...`, corpus
dogfood), merged by hand. This upholds the release-hygiene gate
([issue #5](https://github.com/ralvik/rules_dx/issues/5)): no tags,
releases, or publication outputs without explicit owner approval.
