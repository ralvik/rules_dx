# M11: PATH Tools And Environment Bootstrap

## Outcome

Consumers can bootstrap the module-matched `dx` and configured tools into `.dx/bin` through Bazel.

## Scope

Implement public PATH-tool records, `EnvironmentInfo`, canonical `//dx:env`, host filename validation,
symlink-only atomic `.dx/bin` replacement, management metadata, and initial quality-tool contributions.

## Contract References

- [Environments](../environments/), [CLI](../cli/), [tested platforms](../decisions/0014-tested-platform-release-stack.md), and [testing](../testing/).

## Deliverables

- Public `environment_tool` and `EnvironmentInfo` API.
- Bootstrap env target and managed `.dx/bin` implementation.

## Work Packages

1. Implement validated transitive PATH-tool records and collision detection.
2. Materialize complete staged symlink trees with versioned ownership metadata.
3. Add CLI and initial quality tools using the same public contribution path.

## Milestone-Specific Evidence

- Required hosts prove symlink-only atomic replacement, stale removal, runfiles, signals, and paths with spaces.
- Windows fails before mutation with actionable symlink guidance and no junction/copy/launcher fallback.

## Out Of Scope

- Language-native environments, `dx env` planning, generated-source projection, and `dx setup`.

## Completion Report Additions

- Record API load labels, native filename rules, bootstrap layout, host behavior, and collision cases.
