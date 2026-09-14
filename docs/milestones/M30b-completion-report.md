# M30b Completion Report: Post-Release Adoption (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
no `dx init` output, hook runner, devcontainer build, diagnostics surface,
version launcher, watch loop, inspect wrapper, or completion script exists
yet, so no adoption behavior is established; O49/O50/O51/O55/O56/O61 exact
surfaces remain open under `docs/open-decisions.md`. What lands here is the
pure planning layer the M30b contract authorizes before delivery:
absent-only scaffolding, unmanaged refusal, single-version pinning with
rollback, hermetic-only hook Git, unmanaged-hook overwrite refusal,
two-layer hook status, pinned Bazel-delegated devcontainers, the
rejected-`doctor` diagnostics name, local-only re-resolved watch
iterations, thin inspect forwarding, and single-source completion
generation, all in the zero-dep `dx_adopt` Rust crate unit-tested without
repositories, editors, containers, networks, or shells.

This report closes the planning phase only. Guide prose beyond the M30a
handoff, hook execution with hermetic-Git acquisition, container builds,
diagnostics vocabulary and shape, launcher/self-update mechanics, watch
restart/signal/TTY semantics, inspect query selection, shell-script
mechanics, and any CLI registration are explicitly deferred as
qualification-gated follow-ups (see Open Items). The deferral is
evidence-backed: the owning contracts carry do-not-implement gates until
those qualifications land, and no CLI surface is registered until its
behavior lands.

## WP2: `dx init` Scaffolding (planning; emission deferred)

Init writes absent-only per path (`absent_only_write_allowed`) and refuses
existing paths even with force (`init_must_refuse`), so tracked files and
unmanaged `.envrc` files are never mutated. The empty overlay, gitignore
entry, VSCode settings, and CI caller emission stay delivery-gated; the
refusal shape above is their fixture (slice 1).

## WP3: Hook Runner (planning; execution deferred)

Hook Git is hermetic-only with no ambient fallback
(`hook_git_is_hermetic`); unmanaged shims survive force
(`hook_shim_overwrite_allowed`); status shows the merged baseline, overlay,
and per-check timings (`hook_status_shows_merged`). Trigger shims,
staged-file mechanics, acquisition mappings, and timing evidence stay
gated.

## WP4: Devcontainer, Diagnostics, Versioning (planning; builds deferred)

Devcontainers need pinned bootstrap with Bazel-delegated tools and no
ambient use (`devcontainer_is_admissible`). Diagnostics keeps ADR 0006: no
`dx doctor` under any spelling; the O50 surface ships under another name
(`diagnostics_command_allowed`). The pin holds only when `dx` equals the
module version (`version_pin_matches_module`); rollback re-pins the known
previous only (`rollback_re_pins_previous`). Builds, vocabulary, launcher,
and update mechanics stay gated (slices 1-2).

## WP6/WP7: Watch And Inspect (planning; loops deferred)

Watch iterations need a re-resolved scope, local-only execution, and the
held single-runnable rule (`watch_iteration_accepts`): a thin loop reusing
wrapped commands verbatim, no daemon, cache, graph, or remote. Inspect
allows only nonempty in-workspace scopes (`inspect_scope_allowed`): thin
`query`/`cquery` forwarding with deterministic sorting, no custom graph.
Restart/signal/TTY semantics and query selection stay O55/O56-gated
(slice 3).

## Completion Scripts, Guides, Docs Pipeline (planning; generation deferred)

Completion generates from the single command-definition source;
handwritten per-shell scripts are rejected
(`completion_source_is_single`). Shell list, mechanics, and drift fixtures
stay O61-gated. Guides, examples, and the docs pipeline are the M30a
handoff, consumed unchanged here (slice 3).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success.
- `bazel test //...`: all pass, including `//dx/adopt:dx_adopt_test`
  (12 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_adopt` is a zero-dep pure-planning library fully covered by its
co-located unit tests. No scaffolded files, hook runs, container builds,
diagnostics output, version launches, watch iterations, inspect runs, or
completion scripts; none claimed.

## Changed Components

- `dx/adopt/` (new crate `dx_adopt`): `src/lib.rs` (all three planning
  slices plus 12 unit tests); `BUILD.bazel`, `Cargo.toml`.
- Zero-dep by design: no `MODULE.bazel` manifest changes; no `dx/cli`
  registration (behavior has not landed); no files written, no containers
  built.
- This report.

## Open Items

- Delivery execution (M30b WPs 2-4, 6-7): init emission with absent-only
  enforcement, hook runner with hermetic Git and staged scope,
  devcontainer builds, diagnostics surface (O50), launcher/self-update
  (O51), watch under O55, inspect under O56, completion under O61.
- O49 trigger/staged/ownership details, O55 exact values and restart
  semantics, O56 syntax and query selection, O61 shell list and fixture
  shape; platform evidence per surface.
- First-hour budget proof (clean-machine quickstart to green CI within the
  hour, timed and recorded) stays with delivery, reusing the M30a corpus.
- Milestone exclusions respected: no editor extensions, no remote-dev
  hosting, no quality/generation/environment behavior changes. All
  milestones M00-M30b now hold recorded planning reports; execution and
  release qualification remain gated as recorded above.
