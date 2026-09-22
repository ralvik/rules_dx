# README Split Plan (Issue #926)

Accepted. Grandfathered long READMEs with a split/trim plan so the
60-line gate stays honest without silent drift.

## Grandfathered (over 60 lines, pinned by `bazel run //tools/ci:docs_testing_gates`)

- `docs/testing/README.md` (310 lines): split into per-matrix pages under
  `docs/testing/` (already started: `cli.md`, `github-ci.md`,
  `generation.md`, `environments.md`, `tools.md`); root keeps goals plus
  links only.
- `docs/architecture/README.md` (261 lines): split per-component pages;
  root keeps overview plus links only.
- `docs/generation/README.md` (143 lines): split per-language pages;
  root keeps overview plus links only.
- `.github/workflows/README.md` (98 lines): trim to workflow index;
  details move to `docs/github-ci.md`.
- `docs/cli/commands/README.md` (84 lines): trim to command index;
  details stay in per-command pages.
- `docs/documentation/README.md` (78 lines): trim to doc index;
  details stay in per-doc pages.
- `README.md` (65 lines): trim to landing plus links; details stay in
  `docs/` and `examples/`.
- `docs/README.md` (64 lines): trim to doc index; details stay in
  domain folders.

## Rule

New READMEs must stay at or under 60 lines. Growing a grandfathered file
without shrinking toward its split fails the gate; shrinking a file off
the allowlist removes it from the allowlist in the same PR.
