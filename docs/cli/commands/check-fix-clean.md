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
succeeds only when every phase succeeds.

Proposed merged `--report` semantics (pending O59 freeze, flagged for review): each
`--report <format>=<destination>` request fans out to every phase verbatim, but phases
render to captures and the umbrella writes one merged document per request to `destination`
after the last executed phase. Merge is per format family with phase order authoritative
and no new report kinds: SARIF 2.1.0 concatenates `runs` in phase order; JUnit XML nests
per-phase `testsuite` elements under one `testsuites` root in phase order; LCOV unions `DA`
records; NDJSON/JSON change streams concatenate in phase order. Any other format fails
closed with `report_failed` — no silent merge. After a stop-on-first-failure, the merged
document contains the executed phases only (completed phases plus the failed phase's
verbatim partial under its own partial-collection rule); unstarted phases contribute
nothing. Multiple `--report` requests merge independently. A `-` (stdout) report
destination fails closed under the umbrella: phases would each claim the reserved stdout
document. Destination write errors are independent operational failures per the
[output protocol](../output-protocol.md). Do not implement against this prose until O59
freezes it. Composition fixtures (executable once the commands land): SARIF run order,
JUnit testsuites shape, LCOV union, stop-prefix content, stdout-destination rejection,
unknown-format rejection, and multi-report independence.

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
