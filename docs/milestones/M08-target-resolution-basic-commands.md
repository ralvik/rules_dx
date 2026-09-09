# M08: Target Resolution And Basic Commands

## Outcome

`dx` resolves Bazel graph scopes and provides transparent build, test, run, and coverage commands.

## Scope

Implement labels, patterns, paths, directories, all-owner file resolution, reverse-dependent test mapping,
and thin `build`, `test`, `run`, and `coverage` workflows with Bazel-owned semantics.
The accepted `run` scope includes label/pattern passthrough and file/directory resolution to
exactly one runnable owner; O52 qualification gates its implementation.

## Contract References

- [Targets decision](../decisions/0002-targets-not-files.md), [CLI](../cli/), [testing](../testing/), and open decision [O44](../open-decisions.md).
- [Run command contract](../cli/commands/build-test-coverage.md#dx-run) and
  [O52](../open-decisions.md): freeze remaining run mechanics before affected implementation.

## Deliverables

- Query adapter, deterministic target resolver, command planner, JUnit and LCOV collection.
- The command planner exposes the single command-definition source that [O61 completion](../open-decisions.md) generates shell scripts from.
- Real-Bazel ambiguity, ownership, and direct-command parity fixtures.
- Single-runnable `dx run` command and forwarding behavior under the qualified command contract.

## Work Packages

1. Implement Bazel-backed label, pattern, path, directory, and owner resolution.
2. Add test/coverage reverse-dependency mapping and error taxonomy.
3. Wire build/test/coverage and standard reports without changing Bazel status.
4. Qualify O52 and implement `run` through the shared resolver and owning command contract.

## Milestone-Specific Evidence

- Fixtures prove no BUILD parsing or recursive source enumeration and preserve every owner.
- Planned invocations and outputs match direct Bazel for success, failure, and empty mappings.
- Run fixtures prove the qualified [command contract](../cli/commands/build-test-coverage.md#dx-run):
  scope passthrough, single-runnable selection, zero/ambiguous failures, application-argument
  forwarding, TTY/signals, and local-only behavior.

## Out Of Scope

- Generate, env, codegen, setup, audit, update, multirun, and language semantics.

## Completion Report Additions

- Record exact queries, ambiguity cases, command parity, and report-format coverage.
- Record O52 qualification and run scope, forwarding, and local-only evidence.
