# Architecture

Status: quality-core plus Rust, Python, JavaScript, and TypeScript foundations on delivered layers only.
See the [support matrix](../product/support-matrix.md) for current status.

`rules_dx` is a Bazel module and developer workflow layer that makes the Bazel graph the common
source of truth for local development, coding agents, and CI. Detailed contracts live in the
linked domain documentation and [decision record index](../decisions/).

## Principles

1. Bazel is the only authoritative graph and action engine.
2. The CLI is a thin, inspectable adapter over stable Bazel interfaces.
3. Non-mutating CI checks are directly executable as Bazel targets or aspect workflows.
4. Bazel analysis determines source ownership, configuration, and transitive inputs.
5. Mutating commands are explicit, and check modes expose the same proposed changes without
   writing. Generation retains Gazelle's native write and merge semantics.
6. Language rulesets and ecosystem resolvers remain authoritative for language semantics;
   `rules_dx` does not build parallel compilers, package managers, or dependency graphs.
7. Domain-specific integrations stay separate until multiple concrete implementations demonstrate
   a reusable boundary.
8. Unsupported, ambiguous, or incomplete graph facts fail closed instead of being guessed.

## Component Flow

```text
developer or coding agent
        |
        v
dx CLI: discovery, normalization, diagnostics, command composition
        |
        +--> Bazel launcher --> build / test / coverage / aspect workflows
        |
        +--> Bazel query and cquery --> ownership and workflow selection
```

The CLI asks Bazel for targets and artifacts, then presents or applies the resulting operations.

## Details

- [Ownership](ownership.md): capability packages, layout, and public boundaries.
- [Facade](facade.md): consumer facade mapping, Rust library boundary, and upstream authorities.
- [Lifecycle](lifecycle.md): activation, selection, execution, foundation sequence, and inspectability.
- [Contradiction catalog](contradiction-catalog.md): which statement wins for each known design conflict.
