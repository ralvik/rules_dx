# README Split Plan (Issues #926/#986)

Completed under #986. All indexes stay at or under 60 lines; details live in
named contracts, not READMEs.

## Completed (pinned by `bazel run //tools/ci:docs_testing_gates`)

- `docs/testing/README.md` (40 lines): goals plus links only; layers, coverage,
  infrastructure, and evidence live in `docs/testing/strategy-details.md` plus the
  per-matrix pages (`cli.md`, `github-ci.md`, `generation.md`, `environments.md`, `tools.md`).
- `docs/architecture/README.md` (44 lines): overview plus principles plus flow only;
  ownership lives in `ownership.md`, facade plus libraries in `facade.md`, lifecycle in `lifecycle.md`.
- `docs/generation/README.md` (41 lines): contracts plus links plus `dx generate` snippet only;
  mapping and lock/runner evidence lives in `foundation-qualification.md`.
- `docs/environments/README.md` (34 lines): contracts plus links plus `dx env` snippet only;
  plan mapping lives in `foundation-qualification.md`.
- `docs/tools/README.md` (16 lines): inventory plus links only;
  tool-graph mapping lives in `foundation-qualification.md`.
- `.github/workflows/README.md` (11 lines): workflow index only;
  policy notes live in `docs/testing/workflow-notes.md`.
- `docs/cli/commands/README.md` (44 lines): command index only;
  scope policy lives in `scope-defaults.md`, behavior stays in per-command pages.
- `docs/documentation/README.md` (22 lines): design plus contract index only;
  pipeline and delivery record lives in `delivery.md`.
- `README.md` (57 lines): landing plus links plus minimal snippets only; details stay in
  `docs/` and `examples/`.
- `docs/README.md` (54 lines): doc index only; Internal planning section dropped (issues own planning).

## Rule

New READMEs must stay at or under 60 lines. The allowlist stays empty; any
over-limit README fails the gate with no grandfather entry.
