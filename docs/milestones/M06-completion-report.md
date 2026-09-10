# M06 Completion Report: CLI Process, Output, BEP, And Apply (WP1-WP3)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (process boundary, discovery, forwarding), WP2 (output modes, diff, BEP
collection), and WP3 (envelope validation, consensus, atomic apply) are
complete per the [M06 milestone](M06-cli-process-output-bep-apply.md). No
silent deferrals. No capability-term transitions are claimed: the five new
`dx_*` libraries are first-party implementation under test, not product
support claims.

## WP1: Process Boundary, Discovery, And Forwarding

`dx/process` (`dx_process`) owns workspace discovery, launcher execution,
safe summaries, and quiet/dry-run behavior behind `Fs` and `Runner` seams;
unit tests inject both, so no test touches disk or spawns processes.

- Discovery (`discover`/`discover_real`): `MODULE.bazel` marks the root,
  legacy `WORKSPACE` resolves to `UnsupportedLegacy`, `--workspace`
  overrides. `pinned_bazel_version` reads the pin surface.
- Launcher: `launcher_argv0()` is `"bazel"`; `WORKFLOW_STARTUP_OPTS` split
  from `Scope`; `describe_scope`/`operation_summary` render safe summaries
  that never carry argv, option values, or environment values.
- Forwarding: `ProtectedFlag`/`build_workflow_argv` keep quality workflows
  on `--keep_going` (`quality_keeps_going`) while build/test/coverage
  preserve fail-fast unless the user forwards the flag.
- Dry run: `dry_run_allows`/`dry_run_guard` permit read-only resolution
  queries only; final workflows fail instead of executing.
- Signals: `UnixSignal` maps interrupt/terminate to `libc::SIGINT`/
  `SIGTERM` with `forward_signal` by pid; `Runner`/`SystemRunner` propagate
  child exit codes.

Process exit mapping (recorded per the milestone's report additions):

| Origin | Code | Source |
| --- | --- | --- |
| Success | `0` | `EXIT_SUCCESS` |
| CLI operational failure | `1` | `EXIT_OPERATIONAL` via `operational_code()` |
| Pre-execution usage/workspace/scope/owner failure | `2` | `EXIT_PRE_EXEC` via `pre_exec_code()` |
| Bazel subprocess | preserved | `subprocess_code(code)` identity, pinned by `exit_mapping_preserves_subprocess_codes` |

## WP2: Output Modes, Diff, And BEP Collection

`dx/output` (`dx_output`) renders text, diff, and NDJSON modes under event
schema v1.0 (`SCHEMA_MAJOR = 1`, `SCHEMA_MINOR = 0`, emitted by `schema()` on
every event object). `stdout_owner` proves stream exclusivity: a stdout
report can never coexist with NDJSON (`SecondStdoutReport`), and text/diff
modes own stdout exactly when no report does. `Threshold` (info/warning/
error) ranks severities; `sort_diagnostics` orders initial before terminal,
then path/range/severity/tool/rule/message bytes. `diagnostic_event` binds
`resolution` (fixed/remaining/not_applied) exactly to initial diagnostics in
mutating mode; `mutation_event` binds per-file `reason` exactly to
`not_applied` records. `write_event` rejects non-event values so prose can
never leak into the machine stream.

`dx/diff` (`dx_diff`) renders Myers edit scripts as unified diffs
(`--- a/`, `+++ b/`, `--- /dev/null` for creates, `@@ -os,ol +ns,nl @@`
hunks, `\ No newline at end of file` markers) with `CONTEXT = 3` lines.
Backtracking resolves snake ties to the previous point and steps by branch,
so repeated content aligns deterministically; newline-only changes promote
to delete-plus-insert pairs.

`dx/bep` (`dx_bep`) streams one newline-delimited JSON build-event stream
incrementally through `ArtifactReader`, resolving the requested output group
through BEP named sets (first definition wins; completion may precede
definition). Only `file://` URIs resolve; remote `bytestream://` fails
before the reader is touched (a counting test double asserts zero reads),
and the collector never walks `bazel-out` or fetches over the network.
Unknown event kinds are ignored for forward compatibility; malformed lines,
dangling set references (`MissingNamedSet` with id and line), URI-less set
files, and unreadable artifacts fail the whole collection. Failed labels
report `success=false` with no artifacts; labels that never requested the
group are skipped. Records sort by label bytes, artifacts by path bytes, so
consensus never depends on arrival order.

Collection memory bounds (recorded per the milestone's report additions):
memory is bounded by the stream index, not by artifact contents. Only
named-set ids with their reported URIs plus one record per matching
completed label are retained while streaming; artifact bytes are read after
the stream ends, one file at a time, so peak memory is the index plus the
collected result bytes.

## WP3: Envelope, Consensus, And Atomic Apply

`dx/apply` (`dx_apply`) gates every mutation on parse, consensus,
validation, and atomic write, in that order.

- Envelope (`envelope.rs`, `ENVELOPE_VERSION = 1`): `parse_envelope`
  rejects invalid JSON, wrong versions, empty operation lists, empty paths,
  and non-lowercase-hex digests; `#[serde(deny_unknown_fields)]` rejects
  unknown keys; `emit_envelope` round-trips canonically.
- Consensus (`consensus.rs`): `merge` requires path-set, content, and
  digest agreement across agents before anything applies; stale or
  disagreeing proposals fail independently with the disagreeing paths
  identified.
- Validators (`validators.rs`): path checks precede content checks and
  digest/existence checks run last. Empty, absolute, empty-segment, and
  `..`-escaping paths fail; `BLOCKED_EXTENSIONS` match case-insensitively;
  content over `MAX_OPERATION_BYTES` (1 MiB) fails; `INJECTION_PATTERNS`
  match case-insensitively; update/create existence and
  `original_sha256` digest mismatches fail against current bytes.
- Applier (`applier.rs`): `apply_envelope` validates each operation, then
  writes via `RealFileSystem` (temporary sibling plus rename, creating
  parents) and runs `HookRunner` post-write hooks. Operations apply in
  envelope order; the first failure aborts the remainder. The filesystem
  and hooks are seams; `FakeFs`/`FlakyFs` doubles cover update, create,
  stale-digest abort (file bytes untouched), hook failure, and read/write
  I/O mapping without disk access.

Filesystem atomicity results (recorded per the milestone's report
additions): the `RealFileSystem` round-trip test writes through a nested
path (parents created), reads back exact bytes, asserts no
`.<name>.dx-apply-tmp` staging file remains beside the target, and asserts
directory reads surface as I/O errors rather than missing files. A bare
file name with no parent directory writes in place. `validation_failure_
aborts_before_write` proves a stale digest leaves the prior bytes intact,
so incomplete collection applies nothing.

## Milestone-Specific Evidence

- Malformed/partial BEP: non-array `files`, missing `files`, URI-less
  entries, null/non-object `completed`, missing label/success, non-array
  `outputGroup`, non-object groups, non-array `fileSets`, id-less file
  sets, and unreadable stream lines each fail with the offending line
  number; unknown/progress/finished events are ignored.
- Remote materialization: `bytestream://` URIs fail as `UnsupportedUri`
  with zero artifact reads.
- Stream exclusivity: `SecondStdoutReport` on conflicting stdout
  ownership; `write_event` rejects prose and schema-less objects.
- Failures: failed targets report `success=false` without artifacts;
  group-less successes are skipped; empty-reason `not_applied` and
  reasoned `applied` mutations are rejected.
- Signals: `signal_number` pins `SIGINT`/`SIGTERM`; exit mapping pins
  subprocess code preservation.
- Mutation independence: stale digests abort before any write with bytes
  intact; disagreeing consensus paths fail without applying; hook
  failures abort with per-file hook errors.

## Commands And Results

- `bazel build //...`: success.
- `bazel test //...`: 55/55 pass (unit, rustfmt, and Clippy aspects for
  all five crates: `dx/process` 39 tests, `dx/output` 33, `dx/bep` 25,
  `dx/diff` 17, `dx/apply` 36).
- Coverage gate: `PASS 9633/9633 executable lines` with no errors
  (`bazel coverage //...` plus `bazel run //tools/coverage:check` with the
  Bazel-declared Rust sources and `tools/coverage/inventory.txt`).
- Corpus dogfood: 28 corpus targets (23 prior plus the 5 new `dx/*`
  `corpus` targets owning each crate's `BUILD.bazel`/`Cargo.toml`),
  aspect build success, 56 `dx_results` protos, evaluator
  `--fail_on warning` rejects 0, ownership audit reports 0 uncovered
  (simulated post-commit over tracked plus new files).
- rustfmt (`1.98.0`, pinned toolchain) and Clippy (`0.1.98`, warnings as
  errors) pass on all touched sources via the crate aspects.

## Changed Components And Public Labels

New crates `dx/process`, `dx/output`, `dx/diff`, `dx/bep`, `dx/apply`
(manifest, build file, implementation, tests each) with per-package
`corpus` targets; `MODULE.bazel`/`MODULE.bazel.lock`/
`rust/hello/Cargo.lock` gain the `serde`/`serde_json`/`sha2`/`libc`
crate set via `CARGO_BAZEL_REPIN=1`; `tools/coverage/inventory.txt` lists
the new eligible sources. One validated whole-file coverage ignore:
`dx/apply/src/lib.rs` (module root of only `mod` declarations and
re-exports, no executable statements; every item covered in its own
module), following the established `reason:`-validated marker mechanics.
No consumer-facing label or support claim is added.

## Documentation Changes And Unresolved Items

This report plus the milestone-index link below. The M06 contracts
([mutation decision](../decisions/0005-mutating-operations.md),
[CLI](../cli/cli-contract.md), [output protocol](../cli/output-protocol.md),
[quality result protocol](../quality/quality-result-protocol.md)) are
referenced unchanged. Open items: non-Linux required hosts stay gaps per
prior reports; quality command planning, target resolution, and repository
workflows remain out of scope for M07/M08.
