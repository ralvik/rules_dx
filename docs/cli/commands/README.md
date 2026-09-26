# Command Reference

Scope policy lives in [Scope Defaults](scope-defaults.md).

## Commands

- [`dx build`, `dx test`, and `dx coverage`](build-test-coverage.md)
- [`dx run`](build-test-coverage.md#dx-run): sequential multirun for explicit labels/patterns, single-runnable for file/directory scopes
- [`dx deploy`](build-test-coverage.md#dx-deploy): single-deployable build+run
- [`dx watch`](watch.md): thin local-only loop over `build/test/run/lint/typecheck/format/check/fix`
- [Quality commands](quality.md): `dx lint`, `dx typecheck`, and `dx format`
- [`dx check`, `dx fix`, and `dx clean`](check-fix-clean.md): sequential quality/generation umbrellas and managed-state cleanup
- [`dx generate`](generate.md); BUILD semantics are defined by the
 generation contracts
- [Environment, codegen, and setup commands](environment-codegen-setup.md)
- [`dx security`/`dx license`, `dx update`, `dx bump`, and `dx bazel`](audit-update-bazel.md): audit/update policy plus the widen-one-requirement edit plus unchanged forwarding
- [`dx docs`](docs.md): build, check, and serve the unified documentation site
- [`dx init` and `dx hooks`](hooks.md): scaffolding and the custom hermetic git-hook runner
- [`dx status` and `dx version`](status-version.md): the consolidated diagnostics surface and single-version
 pin/launcher with rollback
- [Thin inspect wrappers (`owners`/`deps`/`why`)](inspect.md): canonicalized `bazel query`/`cquery`
 forwarding reusing [target resolution](../target-resolution.md) without
 a custom graph engine.
- [`dx completion`](completion.md): generated static shell scripts.
- [`dx migrate`](migrate.md): upgrade-only breaking-change rewrites
 over the generation edit-manifest pattern (`--from`/`--to` plus one
 manifest per major hop and one per full version pair for minor/patch;
 live execution fails closed until the first
 manifest lands under issue #462 with upgrade scope under issue #671,
 inside the parsed final
 registry pinned under issues #457/#462).
- [`dx new` and `dx upgrade`](new-upgrade.md): absent-only per-language
 scaffolding plus the one-shot pin plus migrate plus setup composition
 with dry-run and recovery pointer.

## Excluded Commands

There is no `dx doctor` or `dx configure`. Their intended
behavior is covered by explicit commands or is outside the accepted surface
(failure debugging uses `dx status` plus JSON `bazel_failed` errors, see
[status/version](status-version.md#failure-explainer)). `dx doctor` and
`dx configure` fail as unknown commands suggesting `dx status`. Help is
`dx --help`, `dx <cmd> --help` (including `-h`), plus the `dx help [command]`
verb redirect to the same generated help.
