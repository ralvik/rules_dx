# Scope Defaults

Per-command scope policy and no-scope behavior.
The [Command Reference](README.md) stays short; this document owns the scope record.

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

