# M05: Direct Bazel Dogfood

## Outcome

This repository enforces its initial quality policy through direct Bazel actions before a quality CLI exists.

## Scope

Apply M03-M04 to repository-owned Rust, Starlark, TOML, and Markdown targets and CI, closing source
ownership and native-config gaps discovered by real use.

## Contract References

- [Bazel execution](../decisions/0001-bazel-owns-execution.md), [quality](../quality/), [testing](../testing/), [architecture](../architecture/), and open decisions [O21](../open-decisions.md), [O53](../open-decisions.md), and [O62](../open-decisions.md).

## Deliverables

- Repository target ownership and canonical quality configuration.
- CI invocations for direct check/evaluator paths and a documented local dogfood command.
- Vendored version-matched `.bazelrc` preset with owned project flag groups and enforced freshness.

## Work Packages

1. Bring the complete current repository corpus under Bazel ownership.
2. Enable initial adapters and fix corpus findings through ordinary tool behavior.
3. Enforce direct-Bazel checks in CI and measure action/cache behavior.
4. Adopt the vendored `bazelrc-preset.bzl` preset (pinned per [dependency currency](../decisions/0008-dependency-currency.md)): review every generated flag, reconcile collisions with owned lines, keep project overrides explicit and `user.bazelrc` last; add owned `extra_presets` groups starting with coverage; enable dependency-update automation on this repository for the new pin with reviewable PRs and no auto-merge per O53; prove the update loop (version-bump PR, `preset.update` regen, flag-diff review, full verification).

## Milestone-Specific Evidence

- Clean CI proves no unowned applicable repository file and no ambient tool dependency.
- No-op, narrow-change, config-change, and warm-cache runs establish dogfood behavior.
- `preset.update_test` passes; the vendored flag inventory is reviewed; a version-bump PR exercises the regen-and-review loop without auto-merge.

## Out Of Scope

- CLI result collection or mutation, public support claims, and non-initial source classes.

## Completion Report Additions

- Record dogfooded classes, remaining corpus exclusions, action counts, and cache measurements.
- Record the preset pin, reviewed flag inventory, and update-loop evidence.
