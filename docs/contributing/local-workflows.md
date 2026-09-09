# Local Workflows

## Current Workflow

This checkout contains design documentation only. No Bazel configuration,
build/test targets, or `dx` CLI implementation exists yet. There are no
repository-defined formatter, linter, focused-test, or Bazel-suite commands to
run.

For documentation changes, use the manual structure and relative-link checks
described in [Documentation Checks](../testing/README.md#documentation-checks)
and check whitespace. Record unavailable checks as gaps, not passes. Do not add
temporary checks or tools; quality gates arrive with the real adapters in
M04/M05, as specified by [M00](../milestones/M00-bazel-rust-ci-seed-quality.md).

## Planned Linux-First Bring-Up

M00 starts Linux-first with local-only execution per
[M00](../milestones/M00-bazel-rust-ci-seed-quality.md). Record unavailable
required hosts from
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms)
as gaps at entry; do not claim them. Linux-first is bring-up order, not a scope
reduction: v1 retains full required-host coverage.

## Planned Local-Only Coverage

M00 coverage is local-only. The mandatory
[project coverage gate](../testing/README.md#coverage) must pass at M00 exit;
Codecov service activation and fork-PR handling remain under the
[O14 qualification requirements](../testing/README.md#github-coverage-reporting).
Do not present local reports as service evidence.

## Planned Local Overrides

Future `dx init`/`dx hooks install` setup is planned to create a root
`dx.local.toml` overlay with a `[hooks]` table and add its gitignore entry.
The planned overlay merges per-person over the committed typed `hooks`
workspace-policy section, with both triggers configurable in both layers.
This checkout does not consume the overlay or currently gitignore it.
Exact schema stays under [O49](../open-decisions.md) and the
[hooks contract](../cli/commands/hooks.md); this is not a current setup step.
