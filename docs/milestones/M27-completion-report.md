# M27 Completion Report: Consumer CI (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
there is no qualified reusable workflow, caller template, or first-party
reporter yet, so no working consumer-CI support is established; workflow
APIs/pins, runner/platform mappings, event/ref bindings, thread mechanics,
and permissions remain unqualified under `docs/github-ci.md#qualification`.
What lands here is the pure planning layer the CI contract authorizes before
qualification: check selection, revision/scheduling/isolation planning,
caller composition with reviewed pins, audit rendering, untrusted-input
validation, thread accounting, and preset onboarding discipline, all in the
zero-dep `dx_ci` Rust crate unit-tested without GitHub Actions, a Bazel
server, or any runner.

This report closes the planning phase only. Reusable-workflow YAML, caller
YAML, reporter implementation, runner/platform freezing, numeric thread-limit
freezing, public preset-label freezing, clean-external-consumer execution,
and GitHub execution evidence are explicitly deferred as
qualification-gated follow-ups handed to M28 (see Open Items). The deferral
is evidence-backed: the owning contract carries a do-not-implement gate
until those qualifications land, and no CLI surface is registered until its
behavior lands.

## WP1: Selection, Revision, Scheduling, Supersession, Reporting, Fork/Aggregate, Rerun (planning)

`dx_ci` owns the nine accepted checks before any workflow lands: six
Linux-once (`lint`, `typecheck`, `format`, `generate`, `security-audit`,
`license-audit`) plus three per-platform (`test`, `build`, `coverage`) in
canonical starter order with frozen owning commands (`plan_selection`,
`find_check`). Disabled checks are omitted, never passed; unknown opt-outs
and missing platform lists fail closed; platform spellings pass through
verbatim with no implicit default. Revision planning validates the proposed
test-merge only (head/base recorded, `BlockedOnConflict` distinct from
head-only fallback, draft equals ready, queue takes the combined revision,
opaque branches). Parallel is default, sequential changes overlap only,
failures preserve results, cells own isolated output-bases. Same-PR
supersession only, late callbacks blocked, exact snapshot binding. Threads
use fixed presentation with an injected limit (failure-first deterministic,
no rotation, delete-bot-only/resolve-replied, unmappable never threaded).
`dx-ci` is the stable aggregate; fork execution stays read-only with no
self-approve and no fork-code execution in reporting. Reruns reuse identity,
docs-only runs all checks, no path filters, bounded retries reuse results,
Code Scanning defaults off, coverage hides no gap (slices 1-7).

## WP2: Caller Onboarding And Reviewed Pins (planning)

Starter triggers cover PRs, default-branch pushes, manual dispatch, and
consumer-enabled queues (never feature pushes alone); platform validation
runs over the injected supported set (empty/unsupported fail, verbatim
single/multi including non-Linux); configured no-ops without reports are
not collection failures; base advancement requires a fresh run (slice 8).
Caller composition plans over caller-owned inputs only
(`disabled_checks`, `platforms`, `scheduling_mode`,
`code_scanning_opt_in`) with workflow-owned
execution/parsing/threads/comments, no generated setup command, opaque
`plan_pin_update` (explicit identities, reviewed bumps only) with
`apply_pin_update` preserving customizations (slice 9).

## WP3: Event/Revision Wiring (planning; execution deferred)

Covered by the WP1 revision/supersession/queue rules above at the pure
layer: test-merge identity, conflict-blocked reporting without fork-code
privilege, same-PR supersession scope, queue binding to the combined
revision, stale-result rejection. Event/ref delivery, merge-snapshot/diff
mapping, conflict blocked-status execution, queue lifecycle, cancellation
races, and aggregate required-check bindings stay qualification-gated with
no YAML or GitHub execution claimed.

## WP4: First-Party Reporting (planning; transport/API deferred)

Audit uses the same presentation (no counts-only mode, no disclosure
toggle); severity/acceptance stay opaque verbatim; restricted bodies are
withheld without hiding counts/outcomes; public reporting is not
confidential with no generic-redaction claim (slice 10). Artifacts and PR
metadata are untrusted: exact snapshot binding only (presence never
trusted, stale/foreign rejected); privileged reporting never executes fork
code nor grants secrets; thread accounting is keep-only (delete/resolve
free slots, retained resolved never counts, concurrent runs share one
budget, no fresh allowance) (slice 11). Finding/thread identity freezing,
numeric-limit freezing, API-limit qualification, and transport stay
deferred.

## WP5: External-Consumer Matrix (deferred; planning gates handed over)

No clean-external-consumer or GitHub execution evidence exists; none
claimed. Repository dogfood alone is insufficient per the contract. The
pure gates above (selection, triggers, isolation, rerun/idempotency,
aggregate, thread lifecycle, Code Scanning opt-in, fork approval) are the
handoff fixtures M28 must exercise through clean external consumers.

## WP6: Preset Onboarding (planning; public string deferred)

Runbook shape frozen (`PRESET_RUNBOOK_STEPS`: dependency snippet,
generation target, import block, update loop, bot sample); `dx init`
emission stays M30; preset labels stay opaque with the frozen public string
deferred to workflow qualification; preset-affecting changes ship only in
minor/major with release-note callouts (patch rejects); regen stays manual
and reviewed with no auto-merge per O53; clean-consumer triple
(generation/import/reviewed-regen) required with customizations preserved
(slice 12).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (709 targets).
- `bazel test //...`: 177/177 pass, including `//dx/ci:dx_ci_test`
  (74 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate`: no diffs; `bazel run //dx:generate_check`:
  clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_ci` is a zero-dep pure-planning library fully covered by its
co-located unit tests. No workflow-YAML, reporter-execution,
runner-mapping, numeric-limit, public-label, remote, non-Linux, or
external-consumer evidence; none claimed.

## Changed Components

- `dx/ci/` (crate `dx_ci`): `src/lib.rs` (all twelve planning slices plus
  74 unit tests); `BUILD.bazel`, `Cargo.toml`.
- Zero-dep by design: no `MODULE.bazel` manifest changes; no
  `dx/cli` registration (behavior has not landed); no workflow/reporter
  YAML.
- This report.

## Open Items

- Qualification gate (`docs/github-ci.md#qualification`, open-decision
  register): freeze workflow inputs, caller/pin representation,
  compatibility, release/update mechanics; supported platform and
  runner/OS/arch identities, shared-Linux scope, isolation, cache/resource,
  sequential ordering, coverage/test reuse; event/ref delivery, merge
  snapshots, diff mapping, conflict blocked-reporting, queue lifecycle,
  base advancement, cancellation races, aggregate bindings; finding/thread
  identity, deterministic ordering, numeric limit and accounting, safe
  deletion/resolution, outdated locations, API limits; fork roles/settings,
  untrusted validation, privileged reporting, sensitive content, bounded
  retries, opt-in Code Scanning publication.
- O53: bot scope gate unresolved; preset regen stays manual, no auto-merge.
- O62: default-env membership conflict unresolved; untouched by this report.
- Milestone exclusions respected: no new languages/tools, no release
  qualification (M28), no publication (M29), no adoption scope (M30b).
  M28 handoff: qualified mappings plus the pure gates above as fixtures.
