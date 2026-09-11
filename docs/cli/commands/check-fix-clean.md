# `dx check`, `dx fix`, And `dx clean`

Owning decision: [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md).
Mechanics pending under [O59 and O60](../../open-decisions.md).

## `dx check` And `dx fix`

```text
dx check [scope ...] [-- bazel-options ...]
dx fix [scope ...] [-- bazel-options ...]
```

Thin sequential umbrellas over the existing quality and generation
workflows. Each phase reuses its wrapped command's scope resolution, Bazel
invocation, result collection, mutation, reporting, and exit-status behavior
verbatim:

1. `format` (`--check` under `check`, default under `fix`)
2. `lint` (`--check` under `check`, default under `fix`)
3. `typecheck` (`--check` under `check`, default under `fix`)
4. `generate --check` freshness validation under `check`; mutating `generate` under `fix`

With no scope, the umbrella selects `//...`. File, directory, label, and
pattern scopes map exactly as the wrapped commands map them. The first
required phase failure stops the umbrella; no dependent phase or mutation
starts after failure, and `dx` returns that phase's exit code. `fix` does
not rerun Bazel after applying edits; run `check` again to validate the
resulting workspace.

`--check`, `--output`, `--report`, `--fail-on`, `--quiet`, and `--dry-run`
pass through to each phase. `build`, `test`, `coverage`, `audit`, `update`,
`env`, `codegen`, `setup`, `docs`, `init`, `hooks`, and `bazel` are not
umbrella phases. CI keeps invoking each explicit `--check`; the umbrella is
inner-loop convenience with no scheduling, caching, or daemon behavior.

Provisional umbrella composition (pending O59 freeze, flagged for review): NDJSON framing
reuses existing events with no new kinds — one `command_started`/`command_finished` pair
for `check`/`fix` brackets the verbatim per-phase streams in phase order, and line order
stays authoritative per the [output protocol](../output-protocol.md#ndjson-envelope).
`--output diff` concatenates each completed phase's validated diff in phase order; phases
with no changes contribute nothing. `--fail-on` applies per phase verbatim; the umbrella
succeeds only when every phase succeeds. Merged `--report` file semantics across phases
remain open under O59; do not implement them against this prose.

## `dx clean`

```text
dx clean [--dry-run] [--bazel]
```

Explicit managed-state cleanup. By default it prunes only validated
unselected and unused `.dx` generations and setup records as defined by
[Managed Environment State](../../environments/managed-state.md#retention-and-recovery).
It never deletes `.dx/setups/current`, its selected generations, tracked
sources, BUILD files, Bazel outputs, shell profiles, or global PATH entries,
and it refuses unmanaged or digest-spoofed paths.

`dx clean --bazel` additionally forwards `bazel clean` and prints
dangling-link recovery guidance (`dx setup`, `dx env`, or `dx codegen`).
`--dry-run` lists reclaimable generations and links without deleting.
There is no automatic pruning, age policy, or count limit. CI use is not
supported.
