# Contributing

`rules_dx` is a Bazel developer platform with build targets and a `dx`
CLI executed through Bazel on the Linux x86_64 seed host.
Authoritative product and design contracts live under `docs/`.

Start with the [local workflow](docs/contributing/local-workflows.md#current-workflow)
for current checks and tooling gaps. After the green build, run the
[editor plus direnv plus hooks one-shot](docs/contributing/local-workflows.md#bootstrap)
(`dx setup`, `hooks install`, `direnv allow`); first-hour evidence lives in
[First-Hour Timing](docs/contributing/first-hour-timing.md). Agent procedure lives in the root
[AGENTS.md](AGENTS.md) and applicable scoped instructions.

## Delivery Flow

Planned work lives in GitHub issues only. A failed feasibility gate blocks affected scope
until a scope or design decision resolves it. Record decisions in the owning
ADR or domain contract.

## Working In This Repository

- Shared behavior must not rely on untracked local flags.
- Treat warnings as errors. Follow the [current verification workflow](docs/contributing/local-workflows.md#current-workflow);
  run the repository formatter, linter, focused tests, and Bazel test suite.
- Update focused tests and documentation when behavior or user-facing workflows
  change. Do not edit generated files directly; use the documented generator.
- Keep README navigation concise. Put substantial examples under `examples/` and detailed
  contracts in named domain documents; preserve existing authoritative index locations.
- For documentation changes, update the single authoritative source in place
  and link instead of copying.
- Error handling: library crates (`cli/*`) return typed errors via `thiserror::Error`
  with stable `Display` strings; only binaries adapt to `anyhow` at the edge
  for exit-code mapping. Never use `Result<_, String>` for new library APIs.

## Decisions And Scope

- Accepted constraints live in the [decision records](docs/decisions/README.md).
  Do not silently decide open questions elsewhere.
- Product scope follows the [product scope](docs/product/scope.md) and
  [support matrix](docs/product/support-matrix.md). If implementation requires an architecture
  or product change, stop the affected work and propose the documentation update
  with evidence, alternatives, compatibility impact, and blocked work first.
- Coverage follows the [testing strategy](docs/testing/README.md#coverage): the single exact per-cell gate
  with zero uncovered lines over the versioned cell inventories (one gate per cell, no cross-cell
  union; the pinned `dx coverage --min-coverage 97` flag is the user-facing configurable threshold,
  informational only), source-level ignores with short `policy:` reasons plus reviewer approval validated in CI. Bring-up
  is Linux-first local-only; record unavailable required hosts as gaps.

## Pull Requests

- Use the templates in `.github/` for bug reports, feature proposals, and pull
  requests. Include exact commands, Bazel versions, OS/CPU hosts, and coverage
  evidence where applicable.
- Docs describe as-built behavior only; do not record planned work in `docs/`.
- No tags, GitHub releases, registry submissions, or `dist/`/`release/`
  outputs without explicit owner approval. `dist/` and `release/` are
  git-ignored build outputs, never committed. Published bytes are never
  rebuilt or substituted silently.
- The module stays at `0.0.0`; consumers (including
  `examples/consumer-ci/caller.yml` and `examples/docs-ci/caller.yml`)
  pin reviewed commits, never release tags. Caller pins stay frozen at
  the last qualified commit, stay in sync across both example callers
  (drift fails `//tools/ci:examples_pins_test`), and move only together
  in a deliberate, reviewed bump.
