# Command Reference

## Scope Defaults

Commands that accept graph scope use `//...` when no scope is supplied: build,
test, lint, typecheck, format, audit, coverage, check, and fix. This default is independent of
the current working directory. [Generate](generate.md#invocation-and-scope) defaults to
repository-wide operation with explicit scoped generation gated by [O48](../../open-decisions.md).
Codegen and environment accept no argument for
their generated repository roots or one exact target label; see
[Generated Code](../../environments/codegen.md) and
[Developer Environments](../../environments/environment.md).

A directory scope is recursive: `path/to/dir` maps to `//path/to/dir/...` after
workspace containment and Bazel syntax validation. Exact-package selection remains
available through an explicit label or target pattern such as `//path/to/dir:all`.

For a file scope, lint, typecheck, format, and source audit select every direct
Bazel owner. The CLI does not choose an owner by label order or naming convention.
Results are deduplicated by normalized identity; incompatible edits still abort the
affected file's mutation while valid unrelated files may apply.

File-scoped build also selects every direct owner. `dx run` instead requires
exactly one runnable owner and fails with `ambiguous_runnable` or `no_runnable`
(see [O52](../../open-decisions.md)). Test and coverage map every
direct owner to all transitive reverse-dependent tests in the main workspace,
deduplicate the test labels, and select all of them. They do not treat a non-test
source owner as a test. Mapping uses the unconfigured query graph and may include
tests from inactive configurable branches.

## Commands

- [`dx build`, `dx test`, and `dx coverage`](build-test-coverage.md)
- [`dx run`](build-test-coverage.md#dx-run): single-runnable execution (provisional, O52)
- [`dx watch`](watch.md): thin local-only loop over `build/test/run/lint/typecheck/format/check/fix` (mechanics pending O55; [ADR 0017](../../decisions/0017-dx-watch.md), extended by [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md))
- [Quality commands](quality.md): `dx lint`, `dx typecheck`, and `dx format`
- [`dx check`, `dx fix`, and `dx clean`](check-fix-clean.md): sequential quality/generation umbrellas and managed-state cleanup (see [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md))
- [`dx generate`](generate.md); BUILD semantics are defined by the
  [generation contracts](../../generation/README.md)
- [Environment, codegen, and setup commands](environment-codegen-setup.md)
- [`dx audit`, `dx update`, and `dx bazel`](audit-update-bazel.md)
- [`dx docs`](docs.md): build, check, and serve the unified documentation site (provisional surface)
- [`dx init` and `dx hooks`](hooks.md): scaffolding and the custom hermetic git-hook runner
- Thin inspect wrappers (`owners`/`deps`/`why`): provisional under
  [O56](../../open-decisions.md), reusing [target resolution](../target-resolution.md) without
  a custom graph engine; exact contracts must freeze before implementation.
- [`dx completion`](completion.md): generated static shell scripts (provisional, O61).

## Excluded Commands

There is no `dx doctor` or `dx configure`. Their intended
behavior is covered by explicit commands or is outside the accepted surface.
A breaking-change `dx migrate` codemod is deferred post-v1 under [O57](../../open-decisions.md).
