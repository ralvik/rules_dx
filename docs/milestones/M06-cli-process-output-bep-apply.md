# M06: CLI Process, Output, BEP, And Apply

## Outcome

The `dx` process shell can safely invoke Bazel, stream protocol output, collect requested results, and apply validated edits.

## Scope

Implement workspace discovery, launcher execution, safe summaries, quiet/dry-run behavior, text/diff/NDJSON,
exit and signal propagation, streaming BEP collection, result validation, consensus, and atomic per-file apply.

## Contract References

- [Mutation decision](../decisions/0005-mutating-operations.md), [CLI](../cli/), [quality](../quality/), and [testing](../testing/).

## Deliverables

- Rust process, output, BEP collector, diff renderer, and apply libraries.
- Exact `dx bazel` forwarding and protocol compatibility fixtures.

## Work Packages

1. Implement process boundary, workspace discovery, safe display, and forwarding.
2. Implement public output modes and streaming BEP result collection.
3. Implement complete-envelope validation, candidate consensus, digest checks, and atomic application.

## Milestone-Specific Evidence

- Fixtures cover malformed/partial BEP, remote materialization, stream exclusivity, failures, and signals.
- Mutation tests prove stale/disagreeing paths fail independently and incomplete collection writes nothing.

## Out Of Scope

- Quality command planning, general target resolution, and repository workflows.

## Completion Report Additions

- Record process exit mapping, event-schema version, collection memory bounds, and filesystem atomicity results.
