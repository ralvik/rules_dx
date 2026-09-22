# Command Reference

## Scope Defaults

Single source: `Command::scope_policy` in `cli/cli/src/args/command.rs`
(pinned by `scope_defaults_partition_covers_all_commands`); fallback usage
lines render from `Command::pipe_list` (pinned by
`fallback_usage_registry_is_single_sourced`), so `dx --help`, `dx
<cmd> --help`, the `main`/`pre_exec` fallbacks, and this table cannot drift.

| Command | Policy | No-scope behavior |
| --- | --- | --- |
| `audit`, `lint`, `typecheck`, `format`, `build`, `test`, `coverage`, `check`, `fix`, `migrate` | `default-//...` | Select `//...`; independent of cwd; `--here` (`--cwd` alias) selects `//path/...` (`//...` at root) on all but `migrate`; `--here` never combines with explicit scopes |
| `generate`, `docs` | `default-repo` | Repository-wide operation; `--here` selects the directory tree instead; explicit scopes per page |
| `codegen`, `env`, `setup` | `default-repo\|exact-label` | Repository-wide by default or one exact target label; patterns rejected |
| `run`, `hooks`, `watch`, `owners`, `deps`, `completion`, `new` | `require` | Fail pre-exec (exit 2) without the required positional (`run` needs a target; `hooks`/`watch`/`owners`/`deps` a scope; `completion` a shell; `new` a language) |
| `why` | `require-file+label` | Exactly `<file> <label>`; otherwise exit 2 |
| `deploy` | `require-label` | Exactly one `//pkg:target` label; patterns, files, and directories fail exit 2; deployability via cquery (`DxDeployInfo` or executable, aliases included) |
| `update` | `selector-default-all` | Dependency-set selectors, default all sets; `--check` is the preset stale gate |
| `bump` | `require-selector+version` | Exactly `<set:package> <version>`; otherwise exit 2; one requirement per invocation, never batch |
| `clean`, `status`, `version`, `upgrade` | `reject` | Any positional scope fails exit 2 (`version --pin` takes a flag value, not a scope) |
| `init` | `optional-name` | Zero or one module name (default `my_project`); extra positionals fail exit 2 |
| `bazel` | `passthrough` | Raw args forward unchanged to the Bazel launcher; no scope resolution |

A directory scope is recursive: `path/to/dir` maps to `//path/to/dir/...` after
workspace containment and Bazel syntax validation. Exact-package selection remains
available through an explicit label or target pattern such as `//path/to/dir:all`.
Bare invocations in a subdir stay `//...`: only `--here`/`--cwd` selects the
directory tree, never the no-flag default (pinned by
`here_scope_canonicalizes_and_bare_stays_repo_wide`).

For a file scope, lint, typecheck, format, and source audit select every direct
Bazel owner. The CLI does not choose an owner by label order or naming convention.
Results are deduplicated by normalized identity; incompatible edits still abort the
affected file's mutation while valid unrelated files may apply.

File-scoped build also selects every direct owner. File- or directory-scoped
`dx run` instead requires exactly one runnable owner and fails with
`ambiguous_runnable` or `no_runnable`; explicit labels and target patterns
run sequentially as multirun
(see [dx run](build-test-coverage.md#dx-run)). Test and coverage map every
direct owner to all transitive reverse-dependent tests in the main workspace,
deduplicate the test labels, and select all of them. They do not treat a non-test
source owner as a test. Mapping uses the unconfigured query graph and may include
tests from inactive configurable branches.

## Commands

- [`dx build`, `dx test`, and `dx coverage`](build-test-coverage.md)
- [`dx run`](build-test-coverage.md#dx-run): sequential multirun for explicit labels/patterns, single-runnable for file/directory scopes
- [`dx deploy`](build-test-coverage.md#dx-deploy): single-deployable build+run
- [`dx watch`](watch.md): thin local-only loop over `build/test/run/lint/typecheck/format/check/fix` ([ADR 0017](../../decisions/0017-dx-watch.md), extended by [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md))
- [Quality commands](quality.md): `dx lint`, `dx typecheck`, and `dx format`
- [`dx check`, `dx fix`, and `dx clean`](check-fix-clean.md): sequential quality/generation umbrellas and managed-state cleanup (see [ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md))
- [`dx generate`](generate.md); BUILD semantics are defined by the
  [generation contracts](../../generation/README.md)
- [Environment, codegen, and setup commands](environment-codegen-setup.md)
- [`dx audit`, `dx update`, `dx bump`, and `dx bazel`](audit-update-bazel.md): audit/update policy plus the widen-one-requirement edit plus unchanged forwarding
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
[status/version](status-version.md#failure-explainer)). Help stays flag-only
(`dx --help`, `dx <cmd> --help`); there is no `dx help` verb.
