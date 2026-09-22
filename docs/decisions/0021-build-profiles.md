# ADR 0021: Build Profiles

## Status

Accepted.

## Context

`dx build`, `dx run`, and `dx test` have no shared debug/release
vocabulary. Deploy needs a release default while the inner loop wants a
fast default. Per-command flags without shared configs would fragment
the composition model in
[ADR 0011](./0011-configuration-composition.md), where Bazel execution
flags are execution policy, not behavioral policy. Bazel owns execution
([ADR 0001](./0001-bazel-owns-execution.md)), so the vocabulary must map
to Bazel-native `compilation_mode` values behind stable `dx_*` config
names.

This record covers the shared configs only. The `--debug`/`--release`
CLI flags plus `DX_PROFILE` forwarding plus deploy provider/CLI
remainder is qualified under issue #814 (successor to closed #457-#458)
with fixtures in `cli/cli/tests/fixtures/build_profiles/` via
`bazel run //tools/ci:build_profiles_qualification`.

## Decision

Three accepted Bazel configs live in the vendored preset
(`tools/bazelrc/preset.bazelrc`, reviewed via `tools/bazelrc/src/lib.rs`),
plus two provisional lanes (see Provisional Extension below):

- `dx_debug` maps to `dbg`.
- `dx_dev` maps to `fastbuild`.
- `dx_release` maps to `opt`.

Each is a `build:<config> --compilation_mode=<mode>` line, so
`--config=dx_debug`, `--config=dx_dev`, and `--config=dx_release` apply
to `build`, `test`, `run`, and `coverage` through Bazel config
inheritance. Bare invocations keep today's behavior, which equals
`dx_dev` (`fastbuild`).

Command defaults are `dev` for `build`/`run`/`test` and `release` for
`deploy`. Precedence (flag over target attribute over command default)
and the `DX_PROFILE` forwarding contract are qualified under issue #814
(successor to closed #457), not to this record.

## Provisional Extension

Two provisional lanes reserve stable names without changing behavior:

- `dx_dev_remote` maps to `fastbuild`; remote executor/BES selection stays
  open and remote behavior remains unverified.
- `dx_toolchain` maps to `fastbuild`; per-invocation toolchain flags stay
  explicit until toolchain-resolution flags qualify.

Selecting either lane equals `dx_dev` today. Both are additive only; no
existing invocation changes meaning, and the CLI surface is unchanged (no
new `--remote`/`--toolchain` flags; Bazel-level lanes only). Executor plus
toolchain flag qualification stays open work tracked in GitHub issues, not
claimed here.

## Consequences

- One shared vocabulary backs all commands; no per-command flag drift.
- The configs are additive only; no existing invocation changes meaning.
- The preset generator and `preset.update_test` pin the five mappings;
  a new profile or mode change requires a reviewed regen.
- CLI surface is unchanged by this record.

## Rejected Alternatives

- Per-command flags with no shared configs: rejected, fragments
  composition and invites per-tool drift.
- Two modes only with default release everywhere: rejected, penalizes
  inner-loop build speed.
