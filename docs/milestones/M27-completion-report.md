# M27 Completion Report: Consumer CI (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. GitHub execution and external-consumer runs
were unavailable and are recorded as gaps, not claimed.

Delivered: the versioned reusable consumer workflow, the caller template
(`dx init` emission + checked-in example), first-party reporting ownership
in-workflow (no reviewdog), and the `dx_ci` selection/revision/scheduling
gates. Runner/platform execution, event/ref delivery proofs, numeric
thread-limit freezing, and clean-external-consumer matrix runs remain gaps.

Capability transitions: consumer CI is Dogfooded at the artifact level
(workflow + caller are checked in, pinned, and reviewed through local
gates); no `Supported` claim (requires external-consumer + platform
evidence).

## WP1: Selection, Revision, Scheduling, Supersession, Reporting, Fork/Aggregate, Rerun (delivered artifacts + gates)

`.github/workflows/reusable-consumer.yml` (versioned with releases, pin
`@v0.1.0`) implements all nine checks: six Linux-once (`lint`,
`typecheck`, `format`, `generate`, `security-audit`, `license-audit`) plus
three per-platform (`test`, `build`, `coverage`) over explicit caller
platforms (missing/empty fails closed, no implicit default). Parallel is
default with sequential mode available; failures preserve results and fail
the run without cancelling independent checks. Same-PR supersession via
concurrency group; stable aggregate `dx-ci` for branch protection; fork
runs read-only under `all_external_contributors` approval with privileged
reporting separated from fork code; reruns reuse identity; Code Scanning
off by default with explicit opt-in. `dx_ci` gates (74 tests) pin selection,
triggers, isolation, rerun, aggregate, thread lifecycle, and opt-in rules.

## WP2: Caller Onboarding And Reviewed Pins (delivered)

`dx init` emits `.github/workflows/ci.yml` pinning the qualified reusable
workflow; `examples/consumer-ci/caller.yml` is the checked-in starter
(platforms explicit, no silent example default) with `secrets: inherit`
and reviewed-bump discipline; `examples/consumer-ci/README.md` documents
required repository settings separately (approval policy, branch
protection on `dx-ci`, least-privilege permissions). No generated setup
command; no custom bot service.

## WP3/WP4: Event/Revision Wiring, First-Party Reporting (delivered shape)

PR/draft/queue revision identities, blocked-on-conflict reporting without
fork-code privilege, late-callback protection, compact summaries with one
updated comment per PR, replyable review threads with dedup and human-reply
preservation, audit same-presentation rendering, and untrusted-artifact
validation are implemented as workflow structure plus `dx_ci` gates.
GitHub execution races, API-limit qualification, and numeric thread-limit
freezing remain gaps.

## WP5: External-Consumer Matrix (deferred execution)

No clean-external-consumer or GitHub execution evidence exists; none
claimed. Repository dogfood alone is insufficient per the contract; the
pure gates above plus the checked-in workflow/caller are the handoff
fixtures M28 must exercise through clean external consumers.

## WP6: Preset Onboarding (delivered discipline)

Runbook shape, stability discipline (preset-affecting changes only in
minor/major with release-note callouts), manual reviewed regen with no
auto-merge per O53, and the clean-consumer triple requirement stand as
gates; `dx init` emission is delivered (M30); public preset-label freezing
waits on workflow qualification.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/ci:dx_ci_test`
  (74 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.
- Workflow/caller YAML validated by inspection (no GitHub execution
  claimed); `dx init --dry-run` emits the caller pinning
  `reusable-consumer.yml@v0.1.0`.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate. No runner-mapping, numeric-limit, public-label, remote, non-Linux, or
external-consumer evidence; none claimed.

## Changed Components

- `.github/workflows/reusable-consumer.yml` (new: versioned reusable
  workflow, 9 checks, aggregate `dx-ci`).
- `examples/consumer-ci/caller.yml` + `README.md` (new: starter + settings
  guidance).
- `dx/adopt/src/lib.rs` (`dx init` caller emission pinning `@v0.1.0`).
- `dx/ci/src/lib.rs` (selection/revision/scheduling gates, 74 tests).
- This report.

## Open Items

- Qualification execution (`docs/github-ci.md#qualification`): runner/OS/arch
  identities, event/ref delivery, queue lifecycle, cancellation races,
  aggregate bindings, thread-limit freezing, API limits, fork handoff proofs.
- O53: bot scope gate unresolved; preset regen stays manual, no auto-merge.
- O62: default-env membership conflict unresolved; untouched by this report.
- Milestone exclusions respected: no new languages/tools, no release
  qualification (M28), no publication (M29), no adoption scope (M30b).
  M28 handoff: workflow/caller identities plus the gates above as fixtures.
