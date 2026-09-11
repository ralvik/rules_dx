# ADR 0018: Umbrella Check/Fix And Managed-State Cleanup

## Status

Accepted. Supersedes the `dx check` / `dx fix` rejection in
[ADR 0006](./0006-cli-command-surface.md) and the no-cleanup-command rule in
[Managed Environment State](../environments/managed-state.md#retention-and-recovery).
`dx watch` remains the thin local-only loop defined by
[ADR 0017](./0017-dx-watch.md); this record adds no daemon, cache, or graph.

## Context

[ADR 0006](./0006-cli-command-surface.md) rejected a general `dx check`
command composing unrelated workflows because umbrella commands make mutation
and CI behavior unclear, and type checking has distinct cost, diagnostics, and
fix behavior from linting. Local iteration since then shows a repeated typing
tax: reaching a green tree requires `dx format`, `dx lint`, `dx typecheck`,
and `dx generate --check` in the stabilizing order, and that order has to be
memorized from the hook split in the [hooks contract](../cli/commands/hooks.md).
Consumer CI keeps its nine explicit checks under the
[GitHub CI contract](../github-ci.md#check-selection); the umbrella is an
inner-loop convenience, not a CI replacement.

Managed `.dx` generations are retained without automatic pruning under
[Managed Environment State](../environments/managed-state.md#retention-and-recovery),
with no cleanup command, age policy, or count limit. Unselected prepared
generations accumulate as link trees, and `bazel clean` leaves managed links
dangling with only explicit `dx env` / `dx codegen` / `dx setup` recovery.
The [automatic-workflow policy](../product/scope.md#automatic-workflows) still
forbids destructive cleanup as a side effect; cleanup must be an explicit
command.

## Decision

`dx check` and `dx fix` are thin sequential umbrellas over the existing
quality and generation workflows. They introduce no new analysis, resolver,
graph, or mutation semantics:

- `dx check [scope] [-- bazel-options ...]` runs `format --check`, then
  `lint --check`, then `typecheck --check`, then `generate --check`, reusing
  each command's scope resolution, Bazel invocation, result collection, and
  `--fail-on` / `--output` / `--report` behavior verbatim on every phase.
- `dx fix [scope] [-- bazel-options ...]` runs the same phases in default
  mutating mode with the same per-file atomic apply as the wrapped commands.
  It does not rerun Bazel after applying fixes; a later `dx check` validates
  the resulting workspace as a new request.
- Composition is strictly sequential in the stabilizing order
  `format` → `lint` → `typecheck` → `generate`. Parallel execution is
  rejected: parallel mutators over the same files race digest validation and
  candidate consensus, break authoritative NDJSON phase ordering, and contend
  on the shared Bazel output-base lock without local speedup.
- The first required phase failure stops the umbrella and returns that
  phase's exit code; no dependent phase or mutation starts after failure,
  per the multi-process rule in [ADR 0006](./0006-cli-command-surface.md).
- Scope defaults to `//...` with the same file, directory, label, and
  pattern mapping as the wrapped commands. `build`, `test`, `coverage`,
  `audit`, `update`, `env`, `codegen`, `setup`, and `bazel` are not umbrella
  phases. CI keeps invoking each `--check` explicitly.

`dx clean` is the explicit managed-state cleanup command:

- By default it prunes only validated unselected and unused `.dx`
  generations and setup records. It never deletes `.dx/setups/current`, its
  selected generations, tracked sources, BUILD files, Bazel outputs, shell
  profiles, or global PATH entries, and it refuses unmanaged or
  digest-spoofed paths.
- An explicit `dx clean --bazel` also forwards `bazel clean` and then prints
  dangling-link recovery guidance (`dx setup` / `dx env` / `dx codegen`).
  There is no automatic pruning, age policy, or count limit; retention stays
  opt-in explicit.
- `--dry-run` lists reclaimable generations and links without deleting.

NDJSON parent framing and report merging are qualified under resolved
[O59](../open-decisions.md). The `--bazel` flag shape and in-use detection
mechanics remain pending under O60.

## Consequences

- Help text and tests identify `dx fix` as mutating by default and `dx check`
  and `dx clean` as non-mutating except for `clean`'s explicit managed-state
  pruning.
- CI continues to use each explicit `--check`; the umbrella never changes
  check selection, platform mapping, or aggregate status.
- `dx watch` may wrap `check` and `fix` like any other iteration command
  once [O55](../open-decisions.md) mechanics freeze; it gains no daemon.
- Managed-state retention tests prove current-pointer preservation, refusal
  of unmanaged state, and stale-link recovery through explicit workflows.

## Rejected Alternatives

- Parallel umbrella phases with merged live streams: rejected for digest
  races, nondeterministic ordering, and output-base lock contention.
- A configurable composite that reads workspace policy to pick phases:
  rejected; the phase set is frozen so `check` means the same thing in
  every repository.
- Automatic age/count pruning of `.dx` generations: rejected; cleanup stays
  explicit under the automatic-workflow policy.
- Extending `clean` to Bazel outputs, sources, or shell configuration by
  default: rejected; only managed unselected state is in scope.
