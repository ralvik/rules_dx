# Pull Request

## Milestone / Decision

Link the milestone file and any ADR or open-decision rows.

## Changed Components

List packages, rules, CLI commands, docs.

## Verification

Exact commands and results: formatter, linter, focused tests, Bazel suite,
coverage counts/denominators where applicable. Treat warnings as errors.

Follow the repository formatter/linter entry points
(`bazel run //dx/cli:dx -- lint`, `bazel run //dx/cli:dx -- format --check`),
the [manual documentation checks](../docs/testing/README.md#documentation-checks),
and the [local workflow](../docs/contributing/local-workflows.md#current-workflow).
Record missing tool commands and tests as gaps, not passes.

## Evidence

Platform, cache, hermeticity, remote, laziness, external-consumer results as
applicable. Record gaps explicitly; do not convert missing evidence into
support claims.
