# Contradiction Catalog

Status: accepted triage index, docs-only. It records which statement wins for
each known design conflict and points at the owning fix issue. It makes no
code claim itself; code fixes land in the linked split issues.

Rule (from issue #956): last-agreed ADR, scope edit, or GitHub issue decision wins;
older text and stub code lose. Where dates tie, precedence is
`scope.md` > `architecture/README.md` > ADR > domain doc > code comment >
code behavior. Each resolution is either fix code to match the last-agreed
contract or amend the contract with a new dated decision. Never leave both
claiming truth.

Related umbrellas stay owned by their split issues (#916-#922 plus #925 plus #930, all closed) with follow-ups #959, #961-#964. This catalog is the resolver index, not a duplicate.

| ID | Topic | Source A (older/loser) | Source B (last-agreed winner) | Resolution | Owner |
| --- | --- | --- | --- | --- | --- |
| C1 | Thin CLI vs Bazel authority | Second-graph / BUILD-probe / custom watcher-cache code and comments (pre-2026-09) | [Product Boundary](../product/scope.md#product-boundary) (2026-09-21, #959) + [ADR 0001](../decisions/0001-bazel-owns-execution.md#decision) (Accepted) + [Architecture](../architecture/README.md#component-flow) (2026-09-22, #936) | Fix code to match contract | #918, #919, #920, #922 |
| C2 | One-dep lazy stack vs Bzlmod eager eval | One-dep implies zero module-resolution cost (older scope reading, pre-2026-09-21) | [Activation and Laziness](../architecture/README.md#activation-and-laziness) (2026-09-21, #959) + [ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision) (Accepted fit 2026-09-21, #959) | Amend contract with dated fit; fix code to boundary | #917, #921, #930 |
| C3 | Symlink-only `.dx` plus manual EULA vs hermetic no-prerequisite | No host-prerequisite hermetic reading (older scope/ADR 0014 reading, pre-2026-09-21) | [Managed State](../environments/managed-state.md#installation-and-ownership) (2026-09-21, #959) + [Scope](../product/scope.md#first-release-admission) (2026-09-21, #959) | Amend contract with dated fit; fix UX to fail closed | #917 |
| C4 | Accepted-equals-durable vs provisional exceptions | [Decisions](../decisions/README.md) Accepted-holds-durable rule vs five provisional/pinned exceptions (watch, naming, config API, py prerelease, rust fork; pre-2026-09-22) | [ADR 0017](../decisions/0017-dx-watch.md) (Accepted 2026-09-22, #961 keep-watch) plus #916 single-tracker plan for the remaining four (2026-09-22) plus #961 dated deferrals (2026-09-22) | Amend contracts with dated decisions; watch/naming/config durable, py/rust/currency dated deferrals | #916, #961 |
| C5 | CLI surface: verb count, fix rerun, clean scope, docs, doctor | 29-verb thin reading plus `fix` rerun plus `clean` full plus `docs` placeholder plus `doctor` (pre-2026-09-21) | [Scope Initial Command Evaluation](../product/scope.md#initial-command-evaluation) Thin fit 32-verb (2026-09-22, #951) + [ADR 0006](../decisions/0006-cli-command-surface.md) (Superseded 2026-09-09) + [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md) (Accepted) + [ADR 0020](../decisions/0020-remove-dx-docs-placeholder.md) (Accepted) | Fix code to match contract | #962, #925 |
| C6 | version/status/hooks local consts vs ask-Bazel | Ask-Bazel-for-everything reading of thin CLI (pre-2026-09-21) | [`dx status`/`version`](../cli/commands/status-version.md#dx-status) Accepted fit (2026-09-21, #963): static checks, one-const pin, live query open and never claimed | Amend contract with dated fit; no code change beyond fit | #925 |
| C7 | No-remote-exec plus dual gates vs determinism/coverage goals | 97%/100% dual-gate plus remote-silence reading (pre-2026-09-21) | [Scope](../product/scope.md#product-boundary) local per-cell determinism only (2026-09-21, #964) + [Action Model](../quality/action-model.md#outputs-remote-cache-and-execution) (2026-09-21, #964) + [Testing](../testing/README.md#coverage) single exact gate (2026-09-21, #964) | Amend contract with dated fit; fix gates to single | #964, #919 |

## C1: Thin CLI vs Bazel authority

Conflict: [scope.md](../product/scope.md#product-boundary) (2026-09-21) says
`dx` is a transparent adapter and Bazel owns dependencies, actions, caching,
and execution, while older stub code kept a second graph (`tools/depcheck`
universal import heuristic), CLI BUILD probing
(`cli/cli/src/resolve/packages.rs:29`, `cli/update/src/manifest.rs:10`), and a
custom watcher/cache shape rejected by [ADR 0001](../decisions/0001-bazel-owns-execution.md#rejected-alternatives) (Accepted).

Winner: last-agreed contract (scope.md 2026-09-21 plus ADR 0001 Accepted plus
architecture [Component Flow](../architecture/README.md#component-flow)
2026-09-22). Older text and stub code lose.

Resolution: fix code to match the contract. Accepted. Owned by #918
(registry/ownership consolidation), #919 (pipeline semantics), #920 (runner
fail-closed), #922 (spawner unification). This catalog makes no code change.

## C2: One-dep lazy stack vs eager extension eval

Conflict: the one-dependency reading of
[scope.md](../product/scope.md#first-release-admission) (pre-2026-09-21)
implied adding a foundation costs nothing, while `MODULE.bazel:5-413`
declares every foundation extension up front and Bzlmod resolves all modules
before lazy extension eval, so extension Starlark already fetches manifests.

Winner: last-agreed fit in [architecture](../architecture/README.md#activation-and-laziness)
(2026-09-21, #959) and [ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision)
(Accepted fit 2026-09-21, #959): version-resolution cost (declarations plus
`MODULE.bazel.lock` size plus fixed-manifest/index reads) is distinct from
payload acquisition; payloads download only for analyzed targets.

Resolution: amend the contract with the dated fit; fix code to that boundary.
Accepted. Owned by #917 (per-host no-fetch proof) and #921 (monolith split)
with the continuous guard in #930. Deferred EULA failure never proves
laziness.

## C3: Symlink-only `.dx` plus manual EULA vs hermetic claim

Conflict: [managed state](../environments/managed-state.md#installation-and-ownership)
(2026-09-21) mandates filesystem symlinks on every host with Developer Mode
as a bootstrap prerequisite and no fallback, and Windows EULA acceptance stays
deliberate, while the older hermetic reading of scope plus
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision)
(pre-2026-09-21) suggested no host prerequisite at all.

Winner: last-agreed fit in scope [First-Release Admission](../product/scope.md#first-release-admission)
(2026-09-21, #959) and managed state (2026-09-21, #959): symlink privilege is
a bootstrap prerequisite, not a host-SDK exception; Windows keeps full
hermetic acquisition with no Build Tools or host SDK fallback.

Resolution: amend the contract with the dated fit; fix UX to probe and fail
closed with actionable guidance. Accepted. Owned by #917. Hosts that cannot
grant the capability are unsupported, not fallback.

## C4: Accepted-durable vs provisional exceptions

Conflict: [decision instructions](../decisions/AGENTS.md) say Accepted holds
durable constraints only, yet five durable behaviors rested on Provisional or
pinned-fork exceptions with no retirement owner: watch
([ADR 0017](../decisions/0017-dx-watch.md), Provisional), naming mapping
([ADR 0004](../decisions/0004-naming.md)), config aggregate API
([ADR 0011](../decisions/0011-configuration-composition.md)), Python
prerelease ([ADR 0010](../decisions/0010-python-foundation.md)), Rust fork
([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)).

Winner: last-agreed keep-watch decision in issue #961 (2026-09-22): watch is
Accepted durable in [ADR 0017](../decisions/0017-dx-watch.md) with its thin
local-only exception in [scope](../product/scope.md#automatic-workflows);
naming and config are Accepted durable in ADR 0004/0011; Python prerelease,
Rust fork, and currency pins are dated deferrals with review. The #916
single-tracker plan (2026-09-22) retains only the remaining four.

Resolution: watch/naming/config amended to Accepted durable per #961; py/rust/currency
record dated deferrals with stable-watch/upstreaming/release-gate review per
the admission policy. Owned by #961 (closed by this catalog update); #916
tracks the remaining four deferrals.

## C5: CLI surface

Conflict: older surface readings (pre-2026-09-21) claimed 29 thin verbs with
`fix` rerunning, `clean` covering Bazel outputs, a `docs` placeholder, and a
`doctor`, contradicting the as-built umbrellas and removals.

Winner: last-agreed surface in [scope](../product/scope.md#initial-command-evaluation)
Thin fit (2026-09-21, updated 2026-09-22 by #951 to 32 verbs) plus
[ADR 0006](../decisions/0006-cli-command-surface.md) (Superseded 2026-09-09)
plus [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md)
(Accepted) plus [ADR 0020](../decisions/0020-remove-dx-docs-placeholder.md)
(Accepted): `fix` applies once with no post-apply rerun, `clean` prunes only
validated unselected `.dx` state by default (`--bazel` forwards `bazel
clean`), `docs` is the delivered Bazel-cached site, and `doctor`/`configure`
are unknown suggesting `dx status`.

Resolution: fix code to match the contract. Accepted. Owned by #962 (thin fit)
and #925 (UX remainder). Help is `dx --help` plus `dx <cmd> --help` plus the
`dx help [command]` redirect.

## C6: version/status/hooks local consts vs ask-Bazel

Conflict: the ask-Bazel-for-everything reading of the thin CLI (pre-2026-09-21)
contradicted the as-built local `version` pin, static `status` checks, and
hermetic hook runner.

Winner: last-agreed fit in [`dx status`](../cli/commands/status-version.md#dx-status)
(2026-09-21, #963): toolchain/tools details name their `MODULE.bazel` and
`//quality/artifacts` sources, the pin hint tracks the module version from one
const, checks report statically with no Bazel subprocess so startup and
`--dry-run` stay cheap, and live `query`/`cquery` resolution stays open and is
never claimed. `dx version` stays the single-version pin/launcher with
rollback and skew gate; `dx hooks` stays the hermetic git-hook runner.

Resolution: amend the contract with the dated fit. Accepted. Owned by #925.
No second code fix beyond that fit.

## C7: No-remote-exec plus gates vs determinism/coverage goals

Conflict: older readings combined `no-remote-exec` markers with a 97%/100%
dual gate and a remote-silence claim, contradicting the determinism and
coverage goals in the testing and quality contracts (pre-2026-09-21).

Winner: last-agreed fit (2026-09-21, #964) in [scope](../product/scope.md#product-boundary),
[action model](../quality/action-model.md#outputs-remote-cache-and-execution),
and [testing](../testing/README.md#coverage): pipeline plus evaluator actions
are local-only until remote is qualified (local disk-cache reuse still
applies), determinism is local per-cell only with no cross-cell union and no
remote claim, and the repo gate is the single exact per-cell gate with zero
uncovered lines while `dx coverage --min-coverage 97` is the user-facing
configurable threshold, informational only for the repo verdict.

Resolution: amend the contracts with the dated fit; fix gates to the single
gate. Accepted. Owned by #964 with pipeline semantics in #919.
