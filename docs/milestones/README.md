# Milestone Index

This index is the sole source for milestone order, status, dependencies, common entry and exit
definitions of done, and the common completion report.

## Capability Terms

- **Bootstrap-maintained:** required to develop this repository, narrowly pinned and maintained here,
  but not yet a product integration or support claim.
- **Dogfooded:** used by this repository and its CI through the intended product path; this proves local
  utility, not general consumer support.
- **Adapter-tested:** the integration passes its focused contract, acquisition, and fixture suites;
  cross-platform and external-consumer release qualification may remain.
- **Supported:** the documented capability passes every required platform, external-consumer,
  compatibility, laziness, and release gate. Only release evidence may promote this label.

## Order, Status, And Dependencies

The table records the planned dependencies. `Ready` means dependency-ready, not that entry gates
have passed or implementation is authorized. Independent milestones may proceed in parallel only
within approved scope after their entry gates pass. Known dependency and prerequisite conflicts
remain [cross-document blockers](../open-decisions.md#cross-document-blockers).

Required core, framework, and admitted-foundation scope follows the
[first-release admission policy](../product/scope.md#first-release-admission)
and [ADR 0019](../decisions/0019-first-release-additional-foundations.md):
complete foundations only, evidence-backed deferrals only, tools independent.

O46 freeze state lives in the [minimal required core freeze](../product/support-matrix.md#minimal-required-core-freeze-o46-pre-m00):
light freeze at M00 entry (inventory plus dispositions), remaining per-candidate
mappings/effort/ownership pending O30/O31 in M22/M23. Exact remaining
provider/generator mappings and implementation work packages must be resolved
before their affected implementation. M22/M23 own the admitted
additional-foundation cohorts under [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
O46 may require splitting these specifications and updating this DAG before implementation.
Language packaging, source-build, and focused-patch routes are allowed subject to individual review
under the central policy. This index selects or defers no additional language by
itself, freezes no upstream API, and starts no milestone; dispositions live in
[ADR 0019](../decisions/0019-first-release-additional-foundations.md).

The accepted [consumer GitHub CI contract](../github-ci.md) is assigned to M27 for reusable workflow,
caller onboarding, and first-party reporting delivery; M07 remains repository-only initial dogfood.
Exact CI mappings must be qualified and frozen before affected M27 implementation. M28 requalifies
the integration for release and M29 verifies its public delivery. This is a planning assignment only,
without status changes or new dependencies.

M30 turns the published release into adopted usage: onboarding guides with CI-executed examples,
`dx init` scaffolding, the custom hermetic hook runner, devcontainer support, the consolidated
diagnostics surface, `dx` versioning with rollback, and unified documentation delivery
(`dx docs`). Hook semantics (O49), diagnostics
surface (O50), version mechanics (O51), and documentation mechanics (O54) must be
frozen before affected M30 implementation. Completion mechanics (O61) must be
frozen before the M30b completion delivery. M30 is split: M30a delivers the
release-blocking docs subset (guides, examples, IR, site build, `dx docs`
surface; depends on M27, consumed by M28) and M30b delivers post-release
adoption scope (depends on M29).

| Order | Milestone | Dependencies | Status |
| --- | --- | --- | --- |
| M00 | [Bazel, Rust, And CI](M00-bazel-rust-ci-seed-quality.md) ([report](M00-completion-report.md)) | None | Ready |
| M01 | [Starlark Testing And Tested-Stack Metadata](M01-starlark-testing-tested-stack-metadata.md) ([report](M01-completion-report.md)) | M00 | Ready |
| M02 | [Minimum Rust Wrappers And Providers](M02-minimum-rust-wrappers-providers.md) ([report](M02-completion-report.md)) | M01 | Ready |
| M03 | [Quality Source, Policy, Result, Runner, And Evaluator Core](M03-quality-core.md) ([report](M03-completion-report.md)) | M01, M02 | Ready |
| M04 | [Rust, Starlark, TOML, And Markdown Acquisition And Adapters](M04-initial-quality-adapters.md) ([report](M04-completion-report.md)) | M03 | Ready |
| M05 | [Direct Bazel Dogfood](M05-direct-bazel-dogfood.md) ([report](M05-completion-report.md)) | M04 | Ready |
| M06 | [CLI Process, Output, BEP, And Apply](M06-cli-process-output-bep-apply.md) ([report](M06-completion-report.md)) | M03, M05 | Ready |
| M07 | [Initial Quality CLI And Full Initial Dogfood](M07-initial-quality-cli-dogfood.md) ([report](M07-completion-report.md)) | M06 | Ready |
| M08 | [Target Resolution And Basic Commands](M08-target-resolution-basic-commands.md) | M06 | Pending |
| M09 | [Rust-First Gazelle](M09-rust-first-gazelle.md) | M07, M08 | Pending |
| M10 | [Public Generate, Config Binding, And Result Transport](M10-public-generate-config-binding-result-transport.md) | M09 | Pending |
| M11 | [PATH Tools And Environment Bootstrap](M11-path-tools-environment-bootstrap.md) | M07, M08 | Pending |
| M12 | [Complete Rust Foundation](M12-complete-rust-foundation.md) | M09, M10, M11 | Pending |
| M13 | [Repository-Corpus Quality Closure](M13-repository-corpus-quality-closure.md) | M12 | Pending |
| M14 | [Python Application Foundation](M14-python-application-foundation.md) | M13 | Pending |
| M15 | [Python Quality](M15-python-quality.md) | M14 | Pending |
| M16 | [JavaScript And TypeScript Application Foundation](M16-javascript-typescript-application-foundation.md) | M13 | Pending |
| M17 | [JavaScript And TypeScript Quality](M17-javascript-typescript-quality.md) | M16 | Pending |
| M18 | [Vue](M18-vue.md) | M16, M17 | Pending |
| M19 | [Svelte](M19-svelte.md) | M16, M17 | Pending |
| M20 | [Astro](M20-astro.md) | M16, M17 | Pending |
| M21 | [MDX And Mixed Framework](M21-mdx-mixed-framework.md) | M18, M19, M20 | Pending |
| M22 | [Additional Native Foundations And Toolchain Parity](M22-remaining-native-toolchain-parity.md) | M13, M15, M17, M21 | Pending |
| M23 | [Additional Managed Foundations And Runtime Parity](M23-remaining-managed-runtime-parity.md) | M15, M17, M21, M22 | Pending |
| M24 | [Parity Closure](M24-parity-closure.md) | M22, M23 | Pending |
| M25 | [Codegen, Environments, And Setup](M25-codegen-environments-setup.md) | M12, M14, M15, M16, M17, M21, M24 | Pending |
| M26 | [Audit And Update](M26-audit-update.md) | M24 | Pending |
| M27 | [Consumer CI](M27-consumer-ci.md) | M25, M26 | Pending |
| M28 | [Stabilization And Release Qualification](M28-stabilization-release-qualification.md) | M27, M30a | Pending |
| M29 | [Release Publication](M29-release-publication.md) | M28 | Pending |
| M30a | [Adoption Docs (Release-Blocking)](M30-adoption-bootstrap-first-hour.md#m30a-release-blocking-docs) | M27 | Pending |
| M30b | [Adoption Scope (Post-Release)](M30-adoption-bootstrap-first-hour.md#m30b-post-release-adoption) | M29 | Pending |

## Common Entry Definition Of Done

- Each listed dependency has a recorded completion report.
- Blocking decisions and required contract changes are resolved or explicitly identified as gates.
- The worktree and current tested-stack metadata are inspected; unrelated concurrent changes remain
  untouched.
- Work packages, focused verification, and expected support-term transitions are understood before
  implementation begins.

## M00 Entry Checklist

M00 has no dependencies, so entry is the prerequisites below. Checked items are
met in the current docs; unchecked items gate implementation.

- [x] O46 minimal required core/framework inventory frozen and recorded in the
  [pre-M00 freeze](../product/support-matrix.md#minimal-required-core-freeze-o46-pre-m00).
  Admitted-foundation dispositions are decided in
  [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
  per-candidate mappings, effort, and ownership remain pending O30/O31 in M22/M23.
- [x] Required platforms enumerated once in the
  [required-platform table](../decisions/0014-tested-platform-release-stack.md#required-platforms).
- [x] Coverage measurement and ignore-validation mechanics resolved in
  [coverage](../testing/README.md#coverage) (O47 resolved); feasibility evidence runs inside M00 against the seed Bazel.
- [ ] O14 seed candidates rechecked and pinned from executed runs (provisional entry only):
  Bazel `9.2.0`, Bazelisk `v1.29.0`, `rules_cc 0.2.22`, `rules_rust 0.74.0` are
  research observations, not pins, per [O14](../open-decisions.md);
  Rust toolchain is the `rules_rust` default as resolved at implementation
  (observed floor `1.89`). Entry gate passes when exact pins/checksums are
  frozen from runs on the seed host.
- [x] Minimal CI seed host named: Linux x86_64 glibc, local-only; remaining hosts and CI
  qualification deferred to M27, unavailable hosts recorded as gaps, not claimed.
- [x] M00 work packages, verification, and recorded gaps understood: no temporary quality
  checks (real adapters land in M04/M05), Prettier deferred, Linux-first local-only bring-up
  with full required-host coverage retained.
- [ ] Worktree inspected at entry: confirm docs-only checkout state (no `MODULE.bazel`
  yet), host Bazel/Bazelisk presence, and `rustc` absent (Rust arrives via the
  `rules_rust` toolchain). Record the actual revision and dirty state in the
  work report. Unrelated concurrent work preserved.

## Common Exit Definition Of Done

- Every in-scope deliverable and work package is complete; required v1 scope is not silently deferred.
  Additional-foundation deferrals follow [first-release admission](../product/scope.md#first-release-admission).
- Applicable contract suites under [Testing](../testing/) pass, including real Bazel, clean external
  consumer, required-platform, hermeticity, cache, remote, laziness, and deterministic-output evidence
  where those properties are claimed.
- Formatter, linter, generated-file, documentation, and focused regression checks pass with warnings
  treated as errors, for workflows implemented by that milestone. Missing quality gates before
  M04/M05 are accepted gaps recorded as such, not failures.
- The mandatory [project coverage gate](../testing/README.md#coverage) passes for the complete
  first-party implementation: 100% of non-ignored executable lines, not only changed code, with
  source-level ignore directives and nearby explanatory reasons checked in CI. Missing required
  measurement is a blocker; Starlark evidence follows the central instrumentation-first policy.
- Public behavior and workflows are documented at their owning domain paths; generated files are
  changed only through their generator.
- Capability labels use the definitions above. No adapter is called supported before release-level
  evidence exists.
- The completion report is recorded. Recording closes only this milestone.

## Common Completion Report

Every report records completed and deferred scope; capability-term transitions; public APIs and load
labels; changed components; exact commands and results; test, platform, cache, hermeticity, remote,
laziness, and external-consumer evidence as applicable; [coverage evidence](../testing/README.md#coverage),
including counts, denominators, inventory, ignore/reason validation, uncovered or unmeasured items,
and any explicitly labeled Starlark behavioral fallback with its infeasibility evidence; benchmark
results; deviations; unresolved decisions; documentation changes; and confirmation that milestone
exclusions and contract boundaries were respected. It identifies any failed or unavailable evidence
without converting it into a support claim.

Separate evidence-backed additional-foundation deferrals from required-core blockers
and other unresolved required cells under [first-release admission](../product/scope.md#first-release-admission).
A foundation deferral does not dispose of its quality-tool obligations.

## Completion Report Storage

Record each milestone completion report as a per-milestone file under `docs/milestones/`
(for example `M00-completion-report.md`) and link it from the milestone index.
