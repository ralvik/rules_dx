# CLI Test Matrix

This matrix covers CLI and command behavior. Generation-specific command cases are
in [Generation](generation.md), environment/codegen/setup cases are in
[Environments](environments.md), and quality semantics are in
[Quality Workflow Testing](../quality/quality-testing.md).

## Unit Tests

Test argument classification, workspace discovery, command escaping, deterministic
sorting, JSON event encoding, exit-code propagation, signal handling, and query
result interpretation using a fake process boundary. These tests must not encode
BUILD parsing or source scanning.

## End-to-End Tests

Run a real Bazel launcher against fixtures and verify safe operation display, `--quiet`,
label/path selection, workflow selection, mutation/check-mode
separation, failures, logs, dry run, and direct `dx bazel` forwarding. Include
paths with spaces where supported.

Public stream and standard-report tests conform to
[Output Protocol](../cli/output-protocol.md) and [Standard Reports](../cli/standard-reports.md).
Internal analyzer-result tests conform to
[Quality Result Protocol](../quality/quality-result-protocol.md), and acquisition fixtures conform
to [Tool Acquisition](../tools/tool-acquisition.md).

Verify the [no-telemetry contract](../cli/cli-contract.md#responsibilities) by observing
network and process activity during startup, successful commands, handled failures,
crashes, and shutdown. No usage or crash reports may be transmitted directly, through
helpers, or queued for later upload; no opt-in reporting surface may be exposed.
Distinguish permitted acquisition and explicitly configured Bazel services from
telemetry, and verify local diagnostics and requested report files still work.

## Hook Runner

See the [hooks contract](../cli/commands/hooks.md). Verify hooks block staged
violations, pass clean commits, never mutate commits, fetch no ambient tools, and run
only the effective merged configuration. Verify the personal overlay cannot reach CI:
CI enforces the committed baseline with identical results whether or not an overlay
exists locally. Verify budget exceed always blocks and `--no-verify` bypasses are still
fully verified by CI.

Verify staged-path selection uses the hermetically acquired Git executable over worktree
content, never an ambient Git fallback. Exercise a missing or hostile `git` on PATH, managed
Git acquisition failure, and the qualified offline hook setup. Installation must refuse
unmanaged hooks even when force is requested; uninstall removes only owned shims.
Verify `init` can bootstrap without `MODULE.bazel`, writes absent files only, and preserves
all existing content without inspecting Git status. Qualify destination selection and any
managed-file force behavior under O49 before asserting an exact API.

## GitHub Reporting

See the [GitHub CI Test Matrix](github-ci.md) for consumer setup, check selection,
execution, events and revisions, reporting, fork security, merge gating, and qualification.

## Command Registry And Behavior

- Verify the final registry contains exactly `audit`, `lint`, `typecheck`, `test`,
  `format`, `build`, `run`, `watch`, `check`, `fix`, `clean`, `update`, `generate`, `codegen`, `env`, `setup`, `coverage`,
  `docs`, `init`, `hooks`, and `bazel`, plus `owners`, `deps`, and `why` once
  [O56](../open-decisions.md) freezes their inspect-wrapper contracts. Compare the final registry
  with the qualified [command reference](../cli/commands/README.md), not an earlier partial CLI.
- Verify `doctor` and `configure` are rejected as unknown.
- Verify help and machine-readable metadata identify `lint`, `typecheck`, `format`,
  `update`, `generate`, `codegen`, `env`, `setup`, `init`, `fix`, and `hooks` as mutating by default.
- Verify `check` runs `format --check`, `lint --check`, `typecheck --check`, then `generate --check` sequentially with
  `//...` scope default, stops on the first required phase failure with that phase's exit code,
  and starts no dependent phase or mutation after failure. Verify `fix` runs the same sequence in
  default mutating mode, including `generate`, preserving each wrapped command's write semantics
  and no post-apply Bazel rerun per [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md).
  Verify neither umbrella introduces
  parallel execution, caching, scheduling, or daemon behavior.
- Verify update refreshes locked versions to the newest upstream-resolvable versions within declared
  requirements, preserving requirement ranges and exact pins even when newer releases exist outside
  those constraints. Distinguish exact requirement pins from old lockfile selections; an excluded
  newer release is not a failure. Verify prerelease eligibility follows upstream/project policy.
- Verify an upstream Git-dependency fixture advances a branch-based locked commit when the branch
  advances, without changing the declared branch. Exact commit requirements and declared tags must
  remain unchanged even when newer commits or tags exist. Verify the upstream resolver owns the
  resulting lockfile changes and no consumer-worktree Git inspection is introduced.
- Verify update applies without confirmation prompts or an acceptance flag in terminal and
  noninteractive fixtures, without bypassing license consent or mutation safety.
- Verify bare update selects all supported repository dependency sets from both the repository
  root and nested directories, including a mixed-ecosystem fixture with multiple independent
  dependency sets. Selection must not narrow to the containing ecosystem workspace or rely on
  CLI filesystem scanning.
- Verify selective update supports dependency-set selection and individual packages within selected
  sets through real upstream updaters. Include necessary transitive lockfile changes and preserve
  declared requirements. An unsupported package selection must be reported, not silently broadened
  to an update of the entire set. Independent unselected dependency sets must remain unchanged.
- Verify update respects upstream transitive resolution, reports resolver failures or unsupported
  operations without claiming success, and attributes changed files and operations to the
  updater. Do not equate resolver success with application build/test success or introduce a private
  resolver/lockfile. Ordinary build and editor fixtures must not initiate version upgrades.
- Verify a failing dependency-set update does not prevent later independent selected updates:
  preserve successful changes, report every failure, and return an overall failure status. Operations
  depending on the failed update must not run and must be reported as blocked. Include shared-lockfile
  and shared-resolver-workspace fixtures so distinct labels alone do not imply independence; do not
  roll back successful independent updates. Freeze exact aggregate exit codes and per-set
  text/NDJSON reporting under O12 before asserting their mappings; successful updates must not
  hide a failed set or claim blocked dependents succeeded.
- Run every compatibility fixture required by [Output Protocol](../cli/output-protocol.md) for
  each command and supported mode. Cross-command fixtures additionally prove that child
  streams are routed consistently, compact scope semantics match target resolution, and
  no internal/external implementation labels enter structured output.
- Verify workflow commands reject user-supplied external-repository scopes while
  `dx bazel` continues to forward them unchanged.
- Verify `--check` for `lint`, `typecheck`, `format`, `generate`, `check`, and `fix`;
  add [`docs --check`](../cli/commands/docs.md) once [O54](../open-decisions.md) freezes its surface.
- Verify docs build and qualified check modes consume generated, Bazel-cached IR without
  requiring committed snapshots or writing IR beside sources. A cold cache triggers normal
  Bazel execution, not a freshness failure; cached IR is not a source change or mutation event.
- Verify docs check selects extraction and shared validation without rendering, while normal
  build validates then renders. Both reject invalid IR, unresolved links, and missing API references;
  renderer failures are covered by full-build fixtures, not claimed as check-mode coverage.
- Verify `--output diff` is supported by exactly those six commands, reserves stdout for a
  complete unified patch, suppresses `dx` summaries and normalized diagnostics, leaves raw
  child output and operational errors on stderr, and does not alter check, mutation, or exit
  behavior. In mutating mode the patch includes all validated intended changes, including
  paths whose later mutation fails.
- Verify no-scope lint, typecheck, format, audit, check, and fix resolve to `//...` independent
  of the current working directory and do not scan for source files. Verify bare
  `dx audit` runs both `security` and `license` families while each explicit
  family runs alone; qualify the license family under O58 before implementation.
- Verify license expression math: `MIT OR GPL-3.0-only` passes in `distributed`
  while `MIT AND GPL-3.0-only` fails; a `WITH` exception passes only when listed
  verbatim; `UNKNOWN` fails in `distributed` but is inventoried in `internal`.
  `MIT OR AGPL-3.0-only` passes by choosing MIT, while lone `AGPL-3.0-only`
  fails in both tiers via `[policy.blocked]`.
- Verify the `internal` tier never fails on the allow/review/deny table but
  fails on `[policy.blocked]`; unknown `[distribution]` labels fail validation,
  and license exceptions follow the same expiry/obsolescence/visibility
  lifecycle as vulnerability exceptions.
- Verify unlisted SPDX identities and final `review` outcomes fail in `distributed` without
  explicit approval; listing an identity as `review` alone cannot pass it. Test valid explicit
  approval and its visible reporting. An allowed base license must not approve an unapproved
  `WITH` expression. Retain internal inventory-only behavior except blocked policy.
- Verify a license exception survives an upgrade inside its bounded range while the finding
  still applies and the exception is unexpired. Out-of-range versions do not inherit approval;
  expired exceptions and exceptions with no applicable finding fail validation. Do not freeze
  shared-lock tier expectations until O58 resolves attribution.
- Verify target-scoped dependency audit checks complete owning dependency locks, including a
  vulnerable locked package unused by the selected target, without including unrelated dependency
  sets. Multiple targets sharing an owner must not duplicate the same audit context. Bare audit
  covers all applicable repository sets, while source-audit selection retains its existing scope.
  Exercise authoritative target-to-owner mappings rather than CLI source or lockfile discovery.
- Verify a selected unassessed dependency fails audit as incomplete with its identity and limitation,
  while retaining validated findings from assessed dependencies and marking SARIF unsuccessful.
  Contrast a recognized assessable package with no known advisories against unsupported Git revision
  and unidentified private-package fixtures. An empty findings list or advisory risk exception must
  not hide incomplete assessment; qualify upstream evidence and report mappings under O11.
- Verify dependency audit automatically refreshes applicable upstream advisory data without a
  separate manual command and supplies an identified snapshot to Bazel-owned analysis. With unchanged
  dependency locks, a new advisory snapshot must invalidate the relevant cached analysis and expose
  newly applicable findings. Verify analysis consumes declared snapshot inputs, not an untracked live
  database, and refresh leaves dependency requirements/locks, pinned auditors, and selected
  environment/codegen projections unchanged. A required refresh failure must fail the audit, report
  unavailable current data, and neither analyze a stale fallback snapshot nor claim the affected set
  is clean. Exercise this with and without an existing cached snapshot. Qualify exact refresh/cache
  mappings under O11 before implementation.
- Verify advisory acquisition downloads databases without sending dependency inventories to
  vulnerability services. Inspect request URLs, bodies, and telemetry using fixtures with distinct
  package identities; package-specific advisory queries and lockfile uploads must not occur. Run
  snapshot matching with network access denied after inputs are acquired. A query-only integration
  must not be admitted as a conforming fallback.
- Verify vulnerabilities with and without available fixed versions are reported and evaluated under
  the same severity threshold and failure policy. An unfixable finding at the failure threshold must
  fail audit absent explicit risk acceptance, not be hidden or downgraded merely for lacking a fix.
  Preserve available upstream remediation information and
  verify audit does not change dependency versions.
- Verify a known applicable vulnerability with no severity rating fails audit by default and reports
  unknown severity without fabricating a rating. A valid explicit risk exception may exempt it while
  retaining its visible accepted status and reason. Missing severity must not silently suppress the
  finding; qualify diagnostic-level, threshold, and report mappings under O11.
- Verify an explicit, explained advisory/dependency exception exempts only its intended finding
  from failure. Another advisory on the same dependency and unrelated affected dependencies must
  remain subject to the normal policy. Missing reasons or invalid exception identities must fail
  validation; exceptions must not bypass refresh or analysis/collection failures. Qualify native
  configuration and advisory-alias matching under O11.
- Verify risk acceptance matches only its explicit version or bounded range, using upstream version
  semantics. A still-vulnerable upgrade outside that range must not inherit acceptance; a matching
  version may be accepted only while the exception is otherwise valid. Reject missing or invalid
  version selections, test range boundaries and advisory aliases, and verify neither update nor audit
  widens or renews exceptions automatically. An exception with no remaining applicable match fails
  obsolete-exception validation.
- Verify accepted vulnerability findings remain visible in normal text output, structured output,
  and SARIF with accepted/suppressed status and their explanation. Preserve identity and severity,
  do not mark them fixed, and exclude them from failure evaluation without exempting unaccepted
  findings or operational errors. Exact protocol/report mappings require O11 qualification.
- Verify vulnerability risk exceptions require valid expiration dates: missing, malformed, and
  expired dates fail validation, and expired acceptance no longer exempts the finding. Exercise
  before/at/after-expiry boundaries and a later invocation reusing unchanged advisory and dependency
  inputs, so cached acceptance cannot hide expiration. Verify renewal is not automatic; qualify
  date/time inputs, native mappings, and cache behavior under O11.
- Verify an unexpired risk exception fails as obsolete after dependency removal or upgrade beyond
  the affected versions, while an exception still matching an applicable vulnerability remains
  valid. Validate against the complete selected owning set and current advisory snapshot, not an
  incomplete result or unrelated unselected owner. Report obsolete entries without deleting them;
  qualify exact advisory identity/alias and applicability mappings under O11.
- Verify no-scope build invokes `bazel build //...` independent of the current
  working directory.
- Verify no-scope test invokes `bazel test //...` independent of the current working
  directory.
- Verify no-scope coverage invokes `bazel coverage //...` independent of the current
  working directory.
- Verify directory scopes map to recursive Bazel patterns without filesystem source
  enumeration, including directories that are not themselves Bazel packages.
- Verify file-scoped quality commands select all direct owners, preserve distinct
  configured contexts, deduplicate identical stable candidates, and reject final-candidate
  disagreement.
- Verify file-scoped build selects every direct owner in deterministic label order.
- Verify file-scoped test and coverage use every direct owner, select all transitive
  reverse-dependent tests in `//...`, deduplicate labels, and return the same set.
- Verify inactive configurable branches may conservatively add tests and are not
  pruned by CLI heuristics.
- Verify explicit test labels and patterns bypass inference, while no owner and no
  mapped tests produce distinct actionable errors.
- Verify ordinary commands, including `dx bazel`, behave identically for
  relevant current workspace bytes whether files are tracked, modified, staged, or
  untracked, without invoking Git. Managed-path ownership conflicts and source-digest
  mismatches must still fail. The only CLI Git-selection exception is the qualified hermetic
  hook workflow tested above; it must not leak into ordinary quality or generation commands.
- Verify file- and directory-scoped `dx run` resolves owners like `dx build` but
  requires exactly one runnable target: zero runnables fail with `no_runnable`
  and multiple runnables fail with `ambiguous_runnable` without choosing by label
  order. Verify labels and patterns pass through, `--` args reach the binary,
  and execution stays local-only. Qualify remaining O52 mappings before implementation.
- Verify `dx watch` only wraps `build`, `test`, `run`, `lint`, `typecheck`, `format`,
  `check`, and `fix` per [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md),
  subject to O55; every other command is rejected. Verify scope is re-resolved each
  iteration, each iteration emits a fresh `command_started`, quality iterations
  preserve converge-and-apply semantics, and interruption forwards signals to the
  active child. Qualify debounce, ignore set, restart, framing, and local-only
  semantics under O55 before implementation.
- Verify dry-run may execute required read-only query/cquery resolution but never
  executes final workflows, actions, or mutations.
- Verify the first required query/preparation failure prevents dependent subprocesses and
  mutations, preserves that process's exit code, and is not replaced by later cleanup or
  already-running independent operation outcomes. Bazel-internal quality `--keep_going`
  remains one subprocess. Distinguish a failed common prerequisite from a failed individual
  update set: the latter follows the independent-update continuation fixtures above.
- Verify workflow commands reject conflicting protected flags, accept duplicate
  required values, preserve unrelated option order, and never weaken BEP/result
  materialization or config selection. Verify `dx bazel` forwards the same flags
  unchanged.
- Verify workflow commands load the workspace `.bazelrc` but ignore system and home
  rc files; verify `dx bazel` preserves normal Bazel rc discovery.
- Verify workspace discovery requires `MODULE.bazel`, reports legacy-only markers
  with migration guidance, and applies the same requirement to `dx bazel`. `dx init` alone
  has the bootstrap exception above; other commands must not inherit it.
- Verify workflow `--` values are inserted as Bazel command options before targets;
  reject startup options and test-binary argument syntax with guidance to use
  `dx bazel`.
- Verify quality workflows force `--keep_going` and reject `--nokeep_going`; build,
  test, and coverage omit it by default and honor an explicitly forwarded value.
- Verify path queries receive supported query options only, final workflows retain
  unrelated configuration options, and unsupported mirroring neither alters query
  argv nor rejects path scope.
