# Contributing

This repository is design-only: no Bazel configuration, build targets, or `dx`
CLI implementation exists yet. Bazel is the planned execution foundation.
Authoritative product and design contracts live under `docs/`.

Start with the [local workflow](docs/contributing/local-workflows.md#current-workflow)
for current checks and tooling gaps. Agent procedure lives in the root
[AGENTS.md](AGENTS.md) and applicable scoped instructions.

## Delivery Flow

The [milestone index](docs/milestones/README.md) defines dependency readiness,
not authorization to start work. Contributor delivery follows its
[common entry gates](docs/milestones/README.md#common-entry-definition-of-done)
and [common exit gates](docs/milestones/README.md#common-exit-definition-of-done).
A failed feasibility gate blocks affected scope until a scope or design decision
resolves it. Record decisions in the owning ADR or domain contract.

## Working In This Repository

- Shared behavior must not rely on untracked local flags.
- Treat warnings as errors. Follow the [current verification workflow](docs/contributing/local-workflows.md#current-workflow);
  run the repository formatter, linter, focused tests, and Bazel test suite once
  their entry points exist.
- Update focused tests and documentation when behavior or user-facing workflows
  change. Do not edit generated files directly; use the documented generator.
- Keep README navigation concise. Put substantial examples under `examples/` and detailed
  contracts in named domain documents; preserve existing authoritative index locations.
- For documentation changes, update the single authoritative source in place
  and link instead of copying.

## Decisions And Scope

- Accepted constraints live in the [decision records](docs/decisions/README.md).
  Unresolved choices live in [open decisions](docs/open-decisions.md); do not
  silently decide them elsewhere.
- First-release scope follows the [product scope](docs/product/scope.md) and
  [support matrix](docs/product/support-matrix.md). If implementation requires an architecture
  or product change, stop the affected work and propose the documentation update
  with evidence, alternatives, compatibility impact, and blocked work first.
- Coverage follows the [testing strategy](docs/testing/README.md#coverage): 100% of non-ignored executable
  lines, source-level ignores with nearby reasons validated in CI. M00 bring-up
  is Linux-first local-only; record unavailable required hosts as gaps.

## Issues And Pull Requests

- Use the templates in `.github/` for bug reports, feature proposals, and pull
  requests. Include exact commands, Bazel versions, OS/CPU hosts, and coverage
  evidence where applicable.
- For milestone work, include the completion-report fields required by the
  [milestone index](docs/milestones/README.md#common-completion-report).
