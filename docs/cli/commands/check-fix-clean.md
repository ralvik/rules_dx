# `dx check`, `dx fix`, And `dx clean`

Owning decision: [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md).
Umbrella mechanics are frozen under [O59](../../open-decisions.md); cleanup mechanics remain
pending under O60.

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

Umbrella composition uses NDJSON framing that
reuses existing events with no new kinds — one `command_started`/`command_finished` pair
for `check`/`fix` brackets the verbatim per-phase streams in phase order, and line order
stays authoritative per the [output protocol](../output-protocol.md#ndjson-envelope).
`--output diff` concatenates each completed phase's validated diff in phase order; phases
with no changes contribute nothing. `--fail-on` applies per phase verbatim; the umbrella
succeeds only when every phase succeeds.

Each `--report <format>=<destination>` request is routed to the phases whose direct-command
registry supports that format. Unsupported phases contribute nothing; a format supported by
no phase fails before execution. Phases render to captures and the umbrella writes one merged
document per request after the last executed phase. In M10 the only supported umbrella report
is SARIF 2.1.0, whose `runs` concatenate in phase order. NDJSON remains `--output json`, not a
standard report. After a stop-on-first-failure, the merged
document contains the executed phases only (completed phases plus the failed phase's
verbatim partial under its own partial-collection rule); unstarted phases contribute
nothing. Multiple `--report` requests merge independently. A `-` (stdout) report
destination fails closed under the umbrella: phases would each claim the reserved stdout
document. Destination write errors are independent operational failures per the
[output protocol](../output-protocol.md). Composition fixtures cover SARIF run order,
stop-prefix content, stdout-destination rejection, unsupported-format rejection, and
multi-report independence.

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
