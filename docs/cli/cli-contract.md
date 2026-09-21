# CLI Contract

## Responsibilities

The `dx` executable discovers the Bazel workspace, accepts labels, target
patterns, files, and directories, translates paths through Bazel graph queries,
selects workflow targets, performs explicitly requested mutations, forwards
arguments, preserves meaningful exit status, and diagnoses missing local
prerequisites. Normal operation reports concise workflow phases without printing
subprocess argument vectors.

It does not parse BUILD files, recursively enumerate lint sources, resolve
dependencies, create actions, execute non-mutating Ruff or Ty checks directly,
maintain an independent cache, or inspect version-control status outside the narrow
hook exception below.

Every command, including `dx bazel`, operates on current workspace bytes and the current
Bazel graph regardless of whether files are tracked, modified, staged, or untracked. The
CLI does not invoke Git or change behavior based on repository status except for hook
management and staged-file selection under the [hooks contract](commands/hooks.md).
All product Git operations for hooks use hermetic managed Git, never ambient Git.
This exception selects paths, not staged bytes for analysis, and does not authorize
general Git status inspection or clean-worktree requirements. `dx bazel` forwards
Bazel arguments unchanged rather than adding version-control
policy. Mutation safety comes from exact source digests, authoritative tool behavior,
managed-path ownership, and atomic commits. A command may still reject a path it must own
when that path contains unmanaged or conflicting state.

`dx` sends no usage telemetry or crash reports, including on startup, failure, or
shutdown, and provides no opt-in reporting mechanism. It does not queue these reports
for later transmission or delegate their transmission to a helper. Local diagnostics
and explicitly requested output/report files remain available. This policy does not
prohibit declared dependency/advisory acquisition or user-configured Bazel remote
execution, caching, and build-event services; those are workflow traffic, not `dx`
telemetry. It does not change transparent `dx bazel` forwarding.

## Invocation Shape

```text
dx [global-options] <command> [scope ...] [-- bazel-options ...]
```

Global options:

| Option | Meaning |
| --- | --- |
| `--workspace <path>` | Override upward workspace discovery |
| `--dry-run` | Resolve and summarize a plan without executing workflows or mutations |
| `--quiet` | Suppress `dx` operation/planning output while preserving subprocess diagnostics |
| `--verbose` | Enable info-level structured diagnostics on stderr via tracing; orthogonal to `--quiet` (summaries vs logs); default stays byte-identical (`warn` only, `RUST_LOG` overrides) |
| `--output text\|diff\|json` | Select concise text, complete unified patches, or versioned machine-readable events (per-command support in [Output Protocol](output-protocol.md); unsupported modes fail fast, never silently ignored) |
| `--report <format>=<destination>` | Write a supported standard report to a file or `-` for stdout; repeatable |
| `--fail-on info\|warning\|error` | Lowest diagnostic severity that makes a quality command fail |
| `--here` (`--cwd` alias) | Select the current directory tree instead of `//...` (cwd-scope commands only: audit/lint/typecheck/format/generate/build/test/coverage/check/fix; `//path/...`, `//...` at the root; cannot be combined with explicit scopes; never implicit) |

These option names are accepted. Only `dx bazel` promises arbitrary unchanged
forwarding. For workflow commands, arguments after `--` are Bazel command options
and are inserted after the Bazel command and before resolved labels. `dx run`
is the narrow exception — arguments after `--` forward to the application binary, not to
Bazel. Bazel startup
options and test-binary arguments are not supported by other workflow commands; use
`dx bazel` when either is required. `dx` does not infer an option class from target
position or reinterpret trailing values heuristically.

`dx --help` and `dx <command> --help` (including `-h`) render
human-readable help from the same CLI grammar that parses invocations.
There is no `dx help` verb; `help` as a command word fails as unknown command.
Help exits `0` on stdout and stays outside machine-output guarantees:
no NDJSON, even with `--output json`. Top-level help lists commands,
flags, scope forms, exit codes, and output modes; per-command help adds
that command's usage and scope notes.

Workflow commands reserve flags required for correctness: aspect selection,
`dx_results`, `@rules_dx//config:workspace`,
`@rules_dx//config:validate`, the BEP destination/format, the build-profile
`--config=dx_*` pin on `build`/`test`/`run`, and output download or
materialization settings needed to consume results. A user-supplied option that
sets a protected flag to a conflicting value is rejected before execution with the
flag name and required policy, but neither the supplied nor generated option value is
printed. Repeating the required value is accepted and canonicalized. Unrelated Bazel
options retain user order and normal Bazel
precedence. `dx bazel` has no protected flags and forwards arguments unchanged.

Workflow commands use Bazel startup options that disable system and home rc files
while retaining the discovered workspace's committed `.bazelrc`. They do not load
ignored `user.bazelrc` or user-specified implicit rc files. An explicit future rc
option requires a separate contract; it is not inferred from the environment.
`dx bazel` preserves Bazel's normal rc discovery because it is the transparent
escape hatch.

Options used during path resolution are forwarded to query only when that Bazel
query command supports them. Target-configuration options may affect the final
workflow without changing the documented conservative unconfigured source-to-test
mapping. `dx` does not silently translate unsupported build options into query
options or reject path scope merely because final-workflow configuration options
are present.

`--fail-on` applies to normalized diagnostics from lint, typecheck, format check,
and audit results. It does not suppress diagnostics or convert execution failures
into success. The default is `warning`: warnings and errors fail while informational
diagnostics pass. There is no `never` value; execution and protocol failures always
fail independently of this threshold.

Check mode evaluates every original finding against `--fail-on`, including guaranteed-fixable
findings because no fix was applied. Default mutating lint/typecheck evaluates only findings
whose resolution is `remaining` or `not_applied`; successfully `fixed` findings do not fail the
completed invocation.

Native analyzer finding exit codes do not directly determine `dx` status. A
completed analyzer action writes `dx_results` and succeeds; `dx` applies the selected
threshold after collecting all completed results. Failed Bazel actions still make
the command fail because they represent execution or result-production failures.
In lint, typecheck, and format check mode, any validated proposed replacement also fails
independently of `--fail-on`; machine output exposes the exact merged per-file edits.
Quality commands pass `--@rules_dx//config:validate=false` so per-result Bazel
evaluators do not stop collection. The CLI maps its `--fail-on` value to the same
severity comparison used by direct Bazel evaluation.

Quality workflows also pass Bazel `--keep_going` so one analyzer infrastructure
failure does not prevent unrelated actions from producing results. Build, test, and
coverage preserve Bazel's fail-fast default unless the user explicitly forwards
`--keep_going`. A conflicting `--nokeep_going` is protected and rejected for quality
workflows because it weakens result collection.

Direct Bazel CI requests `dx_results` with
`--@rules_dx//config:validate=true` and may set
`--@rules_dx//config:fail_on=info|warning|error`. The default is `warning`.
Each result is evaluated independently; `--keep_going` is recommended when CI wants
Bazel to continue unrelated evaluator actions after a policy failure. An evaluator also
fails when its result proposes any replacement, independently of diagnostic severity.

## Workspace Discovery

Starting from the working directory, `dx` walks ancestors looking for a supported
Bazel workspace marker. `MODULE.bazel` is the only supported marker. A legacy
`WORKSPACE` or `WORKSPACE.bazel` without `MODULE.bazel` produces an actionable error
requiring Bzlmod migration; `dx` does not provide a second dependency setup path.
This applies to every command, including `dx bazel`, except that `dx init` may
bootstrap a new repository without an existing `MODULE.bazel`. This narrow exception
does not add legacy WORKSPACE support or bypass discovery for ordinary workflows.
Bootstrap destination mechanics are implemented as specified in the
[init contract](commands/hooks.md#dx-init): absent-only bootstrap writes,
not Git-based tracked-file safety checks.

Discovery is directory traversal for workspace location, not source scanning.
Failure reports every marker considered and suggests `--workspace`.

The primary installation path is `bazel run //dx:env`. The project-owned
cross-platform environment rule exposes the `dx` executable from the consumer's
pinned `rules_dx` module together with configured development tools. No prebuilt
binaries are published yet (distribution qualified under issue #459; no
binaries published yet). When published, v1
destinations are the Bazel Central Registry for the `rules_dx` module and GitHub
Releases for standalone binaries, with install-time publisher-identity verification
(no checksum-only fallback). The CLI and rules module share
[Semantic Versioning](../environments/environment.md#distribution) but use the
documented result-schema compatibility policy rather than requiring exact patch-
version equality at runtime.

Dry-run may execute read-only Bazel `query` or `cquery` subprocesses needed to
resolve path ownership and workflow targets. It reports safe discovery and resolved-plan
summaries unless `--quiet` is present, but never prints subprocess argv or forwarded
option values. Machine-readable mode emits the same structured operation summaries. It
never runs the final build, test, coverage, check, audit, update, generation, or mutation
subprocesses. If resolution itself would require an action, dry-run fails rather than
executing that action.

## Commands

The stable behavior of every command, graph-scope defaults, and excluded commands is
defined in [Command Reference](commands/README.md). This document owns common
invocation, process, and launcher behavior; [Output Protocol](output-protocol.md) owns
public text, NDJSON, and standard-report behavior.

## Operation Display

Before launching a process, normal text mode prints a concise operation summary, for
example:

```text
Running lint analysis for 3 targets
```

Summaries may contain the `dx` command, workflow phase, resolved target count, and whether
the scope is repository-wide or exact-target. They never contain executable argv,
forwarded option names or values, environment values, request headers, endpoint user
information, or a reconstructed shell command. `--quiet` suppresses these summaries but
does not suppress Bazel or tool diagnostics. `--verbose` is orthogonal: it enables
info-level tracing diagnostics on stderr without changing summaries, stdout ownership,
or machine-output contracts. `--quiet` plus `--verbose` means quiet summaries with
verbose logs; `--verbose` combines with `--output text|diff|json` with logs staying on
stderr so stdout stays machine-owned. `--dry-run` emits the same safe summaries
without running final workflows; read-only resolution queries follow the exception under
Workspace Discovery. The original argument vectors are passed directly to subprocesses
and are never rendered by `dx`. This avoids an incomplete secret-detection or redaction
policy.

Operation summaries expose the compact effective Bazel scope without expanding patterns.
Repository scope is `//...`; a directory is its canonical recursive pattern such as
`//src/auth/...`; user-supplied labels and target patterns remain canonical labels and
patterns. File scopes are the exception because they require graph resolution: summaries
list the deterministically sorted canonical owner labels, or mapped test/coverage labels,
that the workflow actually selects. Internal tool, aspect, and implementation dependency
labels are never included.

Structured workflow scopes contain main-workspace labels and patterns only. External
aspect, toolchain, dependency, compiler, and canonical repository labels are internal
implementation details and are omitted. If Bazel fails while processing an external
repository, its detailed diagnostic remains on stderr while NDJSON emits a stable generic
operational code such as `bazel_failed`, without copying the external label into
structured context. A future command that intentionally accepts external scopes requires
an explicit label-representation contract.

## Exit Status

- `0` means success. CLI-detected pre-execution usage, workspace, scope, owner, and empty-
  test-mapping errors use `2`. Quality-policy and other CLI-originated operational
  failures use `1`.
- Except for the `dx update` aggregation exception below, when a required Bazel
  subprocess fails, `dx` returns its exact nonzero exit code. Bazel-
  authoritative commands therefore preserve Bazel's code; quality commands may return
  `1` after successful Bazel execution when normalized findings cross `--fail-on`.
- For ordinary multi-invocation plans, `dx` stops on the first required subprocess failure and
  returns that process's exit code. It starts no dependent operation or mutation after
  that failure. Already-running independent subprocesses are interrupted or allowed to
  settle only as required for safe cleanup; their later outcomes do not replace the first
  required failure. Bazel's internal `--keep_going` behavior remains contained within one
  quality invocation and does not make the CLI launch later dependent subprocesses.
  `dx run` sequential multirun follows this ordinary plan (issue #463; see
  [dx run](commands/build-test-coverage.md#dx-run)).
- `dx update` is the narrow exception: after a selected dependency set fails, continue
  independent selected sets, skip operations dependent on the failed set, preserve
  successful changes, and fail the invocation overall. The
  [update contract](commands/audit-update-bazel.md#dx-update) owns dependency-set
  independence and partial-success semantics. Aggregate exit-code selection,
  backend mapping, and per-set reporting are specified in that contract; live
  resolver-backend execution runs `dx_update::backend` per set with continuation
  in `dx_update::outcome`.
  Ordinary fail-fast behavior, including `dx check` and
  `dx fix`, is unchanged.
- Signals are forwarded to the active Bazel process; interruption should preserve
  conventional shell behavior. On Unix, `dx` re-raises the signal after safe cleanup.

The common mapping and machine-output behavior are defined in
[Output Protocol](output-protocol.md#exit-codes); the update aggregate mapping above
is specified in the [update contract](commands/audit-update-bazel.md#dx-update).

## Machine-Readable Output

`--output json` selects the versioned NDJSON protocol. Stdout is then machine-only and raw
subprocess output moves to stderr. Events describe durable `dx` semantics without argv,
Bazel progress, raw BEP, or original source contents. Exact replacement strings appear only
in `change` events for check and default modes. `--output diff` instead reserves stdout for a
complete human-readable unified patch calculated from those same exact changes; `dx` suppresses
summaries and normalized diagnostics, while child output and operational errors use stderr.
The complete output schema, ordering rules,
error codes, compact scopes, and compatibility policy are defined in
[Output Protocol](output-protocol.md).

## Standard Reports

Commands use `--report <format>=<destination>` for established domain reports. The initial
formats are SARIF 2.1.0 for lint/typecheck/audit, JUnit XML for test, and LCOV for
coverage. Destinations may be files or `-` for stdout; a stdout report conflicts with
NDJSON, while file reports are atomic and may accompany either live mode. Exact format,
stream, validation, and partial-result behavior is defined in
[Standard Reports](standard-reports.md), with live-stream interaction in
[Output Protocol](output-protocol.md#standard-reports).

## Launcher Decision

The repository commits `.bazelversion` and requires a Bazelisk-compatible `bazel`
launcher. Repository development, generated workflow commands, and `dx bazel` use
that launcher path. `dx` does not fall back to a different executable or silently
accept a direct Bazel binary that ignores `.bazelversion`. The pinned Bazel version
is the only supported Bazel version. Bootstrap selects the latest stable release
available at implementation time; later changes to the pin require repository
tests to pass.

## Implementation Hygiene

Hand-rolled core helpers qualified seed-only under issues #315/#395
(`bazel run //tools/ci:helper_qualification`; adopted with fixture evidence:
digest via `hex`/`blake3`/`sha2`, diff via `similar`, SPDX parse via `spdx`,
date calendar via `chrono`, scratch via `tempfile`, dir sizing and walks via
`walkdir`/`ignore`/`globset`, LCOV `SF`/`DA` parsing via `lcov`; stays
hand-rolled with owned reasons: atomic write plus lock via
`std::fs::File::try_lock` plus `tempfile`, path-ladder classifier,
LCOV ignore scanner with inventory plus verdict, SPDX lattice plus date shape gate
plus scratch discipline wrappers; upstream re-evaluation on new crate versions
plus any future migration stays owned gap).

Date engine stays `chrono` under issue #398
(`bazel run //tools/ci:helper_qualification`; `jiff 0.2` spike rejected:
trivial day-granularity gates need no `tzdb`, heavier bundle/tree plus
mechanical churn for a pre-`1.0` single-owner crate; `chronotope#1768`
open/discretionary, `arrow-rs#9183` not urgent; re-evaluate on `jiff 1.0`).

Clap-as-tokenizer legacy error strings qualified seed-only under issue #316
(`bazel run //tools/ci:clap_tokenizer_qualification`; decision: freeze legacy
output as contract with snapshots, not strict clap parsing with auto help;
frozen with fixture evidence: env, codegen shard, env shard, evaluator,
runner, markdown, plus the dx CLI tokenizer preserve legacy hand-loop strings
via `disable_help_flag` plus `allow_hyphen_values` plus
`invalid_token`/`parse_error` mapping for unknown-flag phrasing, hyphen-value
consumption, attached `=value` whole-token echo, and value-parser rejections;
migrate to strict clap parsing with auto help stays owned gap for any future
migration).

Final command registry plus mutating-vs-check semantics pinned under issues
#457/#462 plus #776 plus #786 (`bazel run //tools/ci:cli_contract_qualification`; final registry
holds exactly the 32 parsed commands including `deploy` plus `bump` plus
`migrate` plus `new` plus `upgrade` plus `docs` (`--from`/`--to` with upgrade-only gate plus manifest
selection, issue #462 with upgrade scope under issue #671; absent-only `new` plus
one-shot `upgrade` composition, see [new/upgrade](commands/new-upgrade.md); docs
build/check/serve over the Bazel-cached site, see [dx docs](commands/docs.md)), with `doctor` plus
`configure` rejected as unknown;
help plus `Command::is_mutating_by_default` identify the mutating default,
`--output=diff` stays exactly the six patch producers, and the `check`/`fix`
umbrella stays the sequential `format` then `lint` then `typecheck` then
`generate` phases with stop-on-first-failure and no parallel, caching,
scheduling, or daemon behavior).

Execution/reporting gaps pinned under issue #590
(`bazel run //tools/ci:cli_execution_gaps_qualification`; fixtures in
`cli/cli/tests/fixtures/cli_execution_gaps/` plus `dx_adopt::plan_watch`
plus `dx_process::build_workflow_argv` plus `dx_cli::plan_reports` plus
`dx_update::aggregate`; CLI-only, no Bazel semantics change; seed only, no
Supported claim; silent substitution across commands stays rejected):

- Watch stays 8 watchable (`build`, `test`, `run`, `lint`, `typecheck`,
  `format`, `check`, `fix`) with 22 fail-closed not watchable plus CI
  refusal (see [dx watch](commands/watch.md#execution-gaps)).
- Arg-forwarding stays wont-fix: Bazel startup options (`bazelrc`,
  `home_rc`/`nohome_rc`, `system_rc`/`nosystem_rc`, `output_base`,
  `output_user_root`, `host_jvm_args`, `server_jvm_out`) and test-binary
  args (`test_arg`) are rejected on workflow commands with guidance to use
  `dx bazel`; only `dx bazel` forwards unchanged, while `dx run` forwards
  after `--` to the application binary, not to Bazel.
- Report matrix stays wont-fix: `lint`/`typecheck`/`check`/`fix` accept
  `sarif`, `test` accepts `junit`, `coverage` accepts `lcov`,
  `audit` accepts `sarif`/`spdx`, and every other command has no standard
  report (`format` included); unsupported combos fail fast with
  `UnsupportedFormat`, never silently substituted.
- Parallelism stays wont-fix: `check`/`fix` run
  `format`→`lint`→`typecheck`→`generate` sequentially, `watch` runs one
  iteration at a time, `update` runs per-set sequentially with continuation,
  and `run` multirun runs sequentially in scope order.
