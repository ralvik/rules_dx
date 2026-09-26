# Pull Request

## Linked Issue + ADR/Contract

Link the owning GitHub issue and any ADR or contract docs.

## Changed Components

List packages, rules, CLI commands, docs.

## Verification

Exact commands and results: formatter, linter, focused tests, Bazel suite,
coverage counts/denominators where applicable. Treat warnings as errors.

Follow the repository formatter/linter entry points
(`bazel run //cli/cli:dx -- lint`, `bazel run //cli/cli:dx -- format --check`).
Record missing tool commands and tests as gaps, not passes.

## Evidence

Platform, cache, hermeticity, remote, laziness, external-consumer results as
applicable. Record gaps explicitly; do not convert missing evidence into
support claims.
