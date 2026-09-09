# M30: Adoption, Bootstrap, And First Hour

## Outcome

A developer new to `rules_dx` goes from zero to green CI within the first hour: guided
onboarding, one-command repository scaffolding, hermetic git hooks, a working devcontainer,
clear environment diagnostics, and a self-managed `dx` version.

Delivery splits into release-blocking M30a (consumed by M28) and post-release
M30b; dependencies live in the [milestone index](README.md#order-status-and-dependencies).

## Scope

Quickstart, tutorial, and migration guides plus the root `examples/` corpus. `dx init`
scaffolding (including generated VSCode settings and extension recommendations) and
`dx hooks` management for the custom hermetic hook runner. Hermetic git
operations for the hook workflow (upstream git rules first, managed fallback only if they
fail the workflow). Devcontainer support that bootstraps the pinned toolchain and delegates
all tool execution to Bazel actions with no ambient tools. The consolidated
environment/diagnostics surface (O50) and single-version per-repository `dx` versioning
with rollback (O51: `dx` version equals the pinned `rules_dx` module version). Hook
trigger and staged-content semantics (O49). Unified polyglot documentation delivery
(O54): generated, Bazel-cached documentation IR, per-language adapters, mdBook site build,
and the qualified `dx docs` build/check/serve surface.
Deliver the approved thin local `dx watch` loop and thin `owners` / `deps` / `why` inspect
wrappers, with mechanics gated by O55/O56 and reuse of M08's qualified resolver.
Required-v1 delivery and release sequencing splits M30 into a release-blocking docs subset (M30a,
consumed by M28) plus post-release adoption scope (M30b). Milestone spec and DAG updates
are still pending under O54. See the
[cross-document blockers](../open-decisions.md#cross-document-blockers).

### M30a: Release-Blocking Docs

Unified documentation delivery: guides (quickstart, tutorial, migration) with
CI-executed verification and the `examples/` corpus; generated, Bazel-cached
documentation IR with per-language adapters; the mdBook site build; and the
`dx docs` build/check/serve surface. Depends on M27; consumed by M28.
Evidence: the site artifact/cache fixtures and the nightly extractor
exception's evidence below.

### M30b: Post-Release Adoption

`dx init` scaffolding, the hermetic hook runner, devcontainer support, the
consolidated diagnostics surface (O50), single-version `dx` versioning with
rollback (O51), the thin local watch loop (O55), and the thin inspect wrappers
(O56), plus generated `dx completion` shell scripts with install guidance (O61).
Depends on M29.

## Contract References

- [CLI command surface](../decisions/0006-cli-command-surface.md) (`init`, `hooks` entries),
  [CLI contract](../cli/cli-contract.md) (no-telemetry), and [output protocol](../cli/output-protocol.md).
- [Documentation](../documentation/README.md) (IR, site build, `dx docs` surface).
- [First-release admission](../product/scope.md#first-release-admission) and the
  [minimal core freeze](../product/support-matrix.md#minimal-required-core-freeze-o46-pre-m00).
- [Watch decision](../decisions/0017-dx-watch.md), its
  [ADR 0018 extension](../decisions/0018-umbrella-check-fix-cleanup-clean.md#consequences), and
  [target resolution](../cli/target-resolution.md) for the inspect wrappers.
- Open decisions [O49, O50, O51, O54, O55, O56, and O61](../open-decisions.md).

## Deliverables

- Guides (quickstart, tutorial, migration from existing Bazel setups) and the `examples/`
  corpus; every guide step is CI-executed so docs cannot rot.
- `dx init` and `dx hooks install/uninstall/status` with hermetic hook execution.
  `dx init` also writes absent-only generated VSCode settings and extension
  recommendations pointing at `.dx` projections, checked-in native configs, and
  `.dx/bin`, plus the single-version `dx` pin.
- Devcontainer configuration with pinned bootstrap and Bazel-delegated tools.
- Consolidated diagnostics surface and `dx` version pin/launcher with rollback.
- Unified documentation site delivery: generated/cached versioned IR, per-language adapters,
  mdBook site build, and `dx docs` build/check/serve under O54-qualified checking semantics.
- Thin local watch loop and inspect wrappers under the O55/O56-qualified contracts,
  plus generated `dx completion` scripts under O61.

## Work Packages

1. Write guides and build the examples corpus with CI-executed verification.
2. Implement `dx init` scaffolding (module, `//dx` wiring, config, CI caller, hooks,
   devcontainer, single-version pin, generated VSCode settings and extension
   recommendations; absent-files-only writes), including the empty gitignored
   `dx.local.toml` overlay and its gitignore entry.
3. Implement the custom hook runner: trigger shims (`pre-commit`, `pre-push`), staged-file
   scope over worktree content, hermetic git sourcing, and the two-layer configuration
   (committed baseline plus gitignored personal overlay).
4. Add the devcontainer, the diagnostics surface, and `dx` versioning with rollback.
5. Deliver the unified documentation pipeline and `dx docs` under O54: IR schema
   with compatibility fixtures, per-language adapters, site build, and
   build/check/serve with generated-artifact and cache evidence.
6. Qualify O55 and implement watch by reusing wrapped-command behavior under ADR 0017/0018.
7. Qualify O56 and implement thin inspect forwarding over the M08 resolver, without a new graph engine.

## Milestone-Specific Evidence

- Documentation satisfies the [site artifact/cache fixtures](../documentation/site.md#freshness)
  without committed or source-adjacent IR, shared validation in build/check, no renderer in check,
  and the [nightly extractor exception's evidence](../decisions/0008-dependency-currency.md#rustdoc-extraction-exception).
- A clean-machine run follows only the quickstart and reaches green CI within the hour
  budget, timed and recorded.
- Hooks block a staged violation, pass clean commits, never mutate commits, and fetch no
  ambient tools; hook timing is recorded. A locally relaxed gate still fails in CI, and
  `dx hooks status` shows the effective merged configuration with per-check timings.
- [Hook-runner fixtures](../testing/cli.md#hook-runner) prove hermetic Git without ambient fallback,
  unmanaged-hook refusal even with force, and init bootstrap without a pre-existing module.
- Devcontainer setup uses only pinned artifacts and runs the full quality path.
- Diagnostics identify a broken toolchain, an uncovered platform, and a stale pin with
  actionable output; rollback restores the previous `dx` revision.
- `dx init` on a clean repository writes absent-only VSCode settings pointing at `.dx`
  projections and `.dx/bin` without mutating tracked files; opening the repository needs
  no manual editor configuration for the contracted Rust/Go/Python/TypeScript integrations.
- Watch fixtures satisfy ADR 0017/0018 and O55: per-iteration scope resolution, failure/recovery,
  single-runnable enforcement, signal/TTY behavior, observation filtering, and local-only enforcement.
- Inspect fixtures satisfy O56-qualified query/cquery forwarding, shared resolution,
  deterministic canonical output, and external-scope rejection without custom graph semantics.

## Out Of Scope

- Custom editor extensions beyond the contracted IDE integrations and remote-dev
  hosting beyond the devcontainer definition. Generated VSCode settings and extension
  recommendations written by `dx init` are in scope; custom marketplace extensions are not.
- Product behavior changes to quality, generation, or environment contracts.

## Completion Report Additions

- Record quickstart timing, hook trigger/semantics decisions, devcontainer build evidence,
  diagnostics vocabulary, version-pin format, and any guide/example drift found by
  CI-executed verification.
- Record O55/O56 qualification, watch/inspect delivery evidence, and the required-v1
  delivery/release sequencing blocker disposition before affected work.
