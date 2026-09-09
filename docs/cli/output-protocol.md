# Output Protocol

## Scope

This document specifies the user-facing output API for every `dx` command. The durable
surface is established by [ADR 0006](../decisions/0006-cli-command-surface.md); exact
event schemas and report profiles remain provisional until M06/M10 qualification. It
covers human output, newline-delimited JSON (NDJSON), standard report exports, stream
ownership, ordering, partial results, and operational errors. It does not expose the
Bazel Build Event Protocol (BEP) or the internal action-result Protobuf described in
[Quality Result Protocol](../quality/quality-result-protocol.md).

The protocol has four outputs:

- Text is the default live interface for people.
- Diff is a complete human-readable unified patch for reportable validated workspace changes.
- NDJSON is the common live machine interface for every command.
- Standard reports are complete domain documents such as SARIF, JUnit XML, and LCOV.

Selecting a standard report does not change workflow execution, diagnostics, mutation
planning, or the underlying workflow result. Failure to validate, write, or commit a
requested report is an independent operational failure and makes an otherwise successful
invocation fail with `report_failed`.

## Stream Ownership

The default `--output text` mode preserves subprocess stdout and stderr. `dx` writes
concise workflow summaries without rendering subprocess argv or forwarded option values.
`--quiet` suppresses those summaries but not Bazel or tool diagnostics.

`--output diff` reserves stdout exclusively for complete UTF-8 unified diffs derived from
validated changes. `dx` suppresses its summaries, notices, and normalized diagnostics rather
than moving them to stderr. Raw subprocess output and operational errors remain on stderr.
The mode is accepted only by lint, typecheck, format, generate, check, and fix. It never changes
execution or mutation behavior: check mode still writes nothing, while default mode still
attempts the same changes. Diff rendering does not rerun a tool, Gazelle, or a comparison
workflow. `--quiet` has no additional effect on the already patch-only `dx` presentation.

`--output json` reserves stdout exclusively for NDJSON. Every non-empty stdout line is
one complete UTF-8 JSON object followed by `\n`. `dx` writes no prose or raw subprocess
bytes to stdout; Bazel and tool stdout and stderr remain visible on stderr.
Generate uses its structured Gazelle result manifest in text, diff, and JSON modes. The manifest
contains exact edits and non-mutating ignored-import audit records; no mode reruns Gazelle to
produce its output.

A standard report destination of `-` reserves stdout for exactly that report document.
Wrapper summaries and all subprocess output move to stderr. A stdout report conflicts
with `--output diff`, `--output json`, and any second stdout report. File reports may
accompany any live-output mode.

No `dx`-generated summary or structured event prints subprocess argv, reconstructed shell
commands, forwarded option values, environment values, request headers, or endpoint
credentials. The original argv is passed directly to the subprocess. Subprocess output is
untrusted passthrough and may contain anything the child itself prints; users must not
pass secrets to tools that echo them. This rule includes `dx bazel`, errors, and dry-run.

## Text Output

Text output is concise and task-oriented. Wrapper summaries go to stdout and wrapper
warnings/errors go to stderr. Before work starts, a summary may identify:

- The `dx` command and workflow phase.
- The compact effective Bazel scope.
- Counts such as the number of file owners selected.
- Committed mutations and selected environment/codegen generations.
- Actionable errors and standard-report destinations.

Lint and typecheck check mode show every initial diagnostic and mark `fixable` only when an
exact validated fix is guaranteed. Default mutating text shows terminal diagnostics for files
whose candidate applied and initial diagnostics for files left unchanged; it hides initial
diagnostics resolved by an applied guaranteed fix and summarizes applied fixes. Diff mode
renders none of these normalized diagnostics or summaries. The CLI does not infer a
diagnostic/fix association from a file-level change.

For example:

```text
Running lint analysis for //src/auth/...
```

Text output never attempts to be a machine-stable grammar. Automation uses NDJSON or a
standard report.

## Diff Output

Diff mode emits one concatenated unified diff in normalized path UTF-8 byte order. It includes
every change allowed by the collection rules below, without truncation or omission based on a
later mutation outcome. It is the complete patch for those validated change records, not a claim
about final workspace state after a partially failed mutating invocation. An incomplete quality
result or check-mode generation manifest emits no patch. For a structurally valid default-generation
late-failure manifest, the patch may contain only the validated attempted prefix and is not a
repository-wide generation result. Existing files use `--- a/<path>` and `+++ b/<path>` headers.
New files use `--- /dev/null` and `+++ b/<path>`. Paths are emitted literally after protocol
normalization; a path containing a tab, carriage return, or line feed cannot be represented
unambiguously and fails with `invalid_result` before diff output.

Hunks use three context lines, line numbers are one-based, and content lines use the standard
space, `-`, and `+` prefixes. The renderer reconstructs exact candidate bytes from the same
validated edit set used for JSON and mutation, then computes presentation-only line hunks in
memory. It does not write a temporary workspace, invoke an external `diff`, or use patch fuzz.
For a final line lacking a line-feed byte, it emits the conventional
`\\ No newline at end of file` marker. An invocation with no changes writes no bytes to
stdout. The patch stream contains no prose, diagnostics, mutation annotations, or success
filtering. Operational failures remain on stderr and in the process exit status. A failed
mutation does not remove its already calculated file diff.

## Compact Scope

Operation output preserves compact main-workspace Bazel scope rather than enumerating a
pattern's expansion:

- Repository scope remains `//...`.
- A directory remains a canonical recursive pattern such as `//src/auth/...`.
- Explicit labels and target patterns remain canonical labels and patterns.
- A file scope lists the sorted canonical owner labels, or mapped test/coverage labels,
  because those labels are the result of graph resolution.

Internal aspect, toolchain, compiler, dependency, and external-repository labels are not
part of structured scope. Workflow commands reject explicit external-repository scopes.
If Bazel fails while processing an external repository, the detailed Bazel diagnostic
remains on stderr and the structured stream uses a generic operational error.

## NDJSON Envelope

Every event contains these common fields plus its event-specific fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | object | Schema version for the complete invocation |
| `schema.major` | unsigned integer | Breaking semantic version |
| `schema.minor` | unsigned integer | Additive feature version |
| `event` | string | Event kind defined below |

The initial version is `{"major":1,"minor":0}`. One invocation never mixes schema
versions. Within a major version, producers may add fields and event kinds. Consumers
must ignore unknown fields and event kinds. Removing a field, making an optional field
required, or changing existing semantics requires a new major version.

NDJSON line order is authoritative. The initial schema has no run ID, sequence number,
timestamp, or operation ID. Concurrent subprocesses within one invocation are contained
work for safe cleanup under the [CLI contract](cli-contract.md#exit-status) and do not
create interleaved operations needing correlation. A future correlation field may be added as a minor-compatible
change only when stable interleaved operations create a concrete need.

The initial event kinds are:

- `command_started`
- `operation`
- `diagnostic`
- `notice`
- `change`
- `mutation`
- `report`
- `selection`
- `error`
- `command_finished`

Raw Bazel progress and BEP events are never part of this API.

## Command Started

`command_started` is the first event in every successfully initialized JSON stream.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `command` | yes | string | One stable command name from the command registry |
| `dry_run` | yes | boolean | Whether final workflows and mutations are disabled |
| `mode` | yes | string | `default` or `check` |

```json
{"schema":{"major":1,"minor":0},"event":"command_started","command":"lint","dry_run":false,"mode":"default"}
```

Lint, typecheck, format, generate, check, and fix accept `check`.
[`docs --check`](commands/docs.md) is validation without rendering; its exact output mapping
remains pending [O54](../open-decisions.md). Docs build and check do not emit source `change` or
`mutation` events for generated Bazel artifacts. Other commands use `default`.
Subject to the collection and manifest validation rules below, JSON reports each exact calculated
file change in both check and default modes as a `change`. Default mode additionally reports each
terminal file write attempt as a `mutation`. `applied` confirms a committed change; `not_applied`
confirms a failed attempt.
If a valid `--output json` is parsed before a later initialization failure, the stream
contains exactly one `error` followed by one
`command_finished`, without `command_started`. If output mode itself cannot be parsed, no
NDJSON contract has been selected.

## Operation

An `operation` event announces a durable workflow phase before it begins or, under
dry-run, a phase that would have begun.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `command` | yes | string | Owning `dx` command |
| `phase` | yes | string | `discover`, `resolve`, `query`, `execute`, `collect`, `apply`, `report`, or `select` |
| `scope` | no | array of strings | Compact effective main-workspace labels or patterns |

```json
{"schema":{"major":1,"minor":0},"event":"operation","command":"lint","phase":"execute","scope":["//src/auth/..."]}
```

Operation events do not expose Bazel actions, progress counters, aspect labels, output
groups, or subprocess boundaries. Repeated phases are interpreted by line order.

`scope` appears once the effective graph scope is known. A file-scoped `resolve` or
`query` operation omits it; the subsequent `execute` operation contains the resolved owner
or mapped test/coverage labels. Other graph-scoped quality, build, test, audit, and
coverage operations include compact scope immediately. Explicit-target env/codegen/setup
operations include scope; no-argument env/codegen/setup, generate, update, and bazel omit
it. Canonical workflow implementation roots such as `//dx:env` are not emitted unless the
user supplied that label as scope.

## Diagnostic

Each normalized source finding is one `diagnostic` event.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `severity` | yes | string | `info`, `warning`, or `error` |
| `tool` | yes | string | Stable adapter tool identifier |
| `message` | yes | string | Normalized diagnostic text |
| `rule` | no | string | Native or normalized rule identifier |
| `path` | no | string | Normalized workspace-relative source path |
| `range` | no | object | Half-open UTF-8 byte range in `path` |
| `range.start_byte` | when `range` exists | unsigned integer | Inclusive byte offset |
| `range.end_byte` | when `range` exists | unsigned integer | Exclusive byte offset |
| `snapshot` | quality pipeline diagnostics | string | `initial` or `terminal` |
| `fixable` | yes | boolean | Whether the adapter guarantees the validated exact change resolves this finding |
| `resolution` | default mutating initial diagnostic | string | `fixed`, `remaining`, or `not_applied` |

Normalized paths are valid UTF-8, slash-separated, non-empty, lexical, contain no `.` or
`..` component, and remain beneath the main workspace. `path` and `range` are omitted for
findings attributed only to read-only generated context. A range requires a path,
`start_byte <= end_byte`, and both offsets must lie on UTF-8 code-point boundaries within
the identified analyzed source snapshot. An `initial` range addresses the original bytes
identified by the file's source digest. A `terminal` range addresses the reconstructed stable
candidate bytes from its `change` event. The range is not a promise
about post-mutation workspace bytes. NDJSON does not duplicate line or column coordinates.
Text and standard-report rendering derive them only when needed.

Fixability is strictly binary. `true` requires authoritative tool/adapter evidence and an
exact same-file edit in the validated candidate that is guaranteed to resolve the diagnostic.
Every unsupported, ambiguous, merely rule-level, pathless, or unlinked claim is `false`; there
is no unknown value and the CLI never infers fixability from another change on the file.
Terminal diagnostics always use `fixable=false`: a stable complete round means no selected
stage proposed another applicable fix for the terminal state.

Check-mode JSON emits both initial and terminal pipeline findings, including fixable initial
findings, and omits `resolution` because no mutation was attempted. Default mutating JSON also
emits both snapshots. Every initial diagnostic requires `resolution`: `fixed` means the
finding is absent from the terminal snapshot and that file's candidate applied;
`not_applied` means it is absent from the terminal snapshot but the candidate did not apply;
`remaining` means the finding is also present in the terminal snapshot. Terminal diagnostics
omit `resolution`; their `snapshot` and the file's mutation outcome state whether they describe
the resulting workspace or an unapplied candidate. A fixed diagnostic does not participate in
default-mode `--fail-on`; terminal findings for applied candidates and initial findings for
unapplied candidates do.
`not_applied` includes a candidate rejected before its write attempt and one prevented by a
global collection/protocol failure. It does not claim that a mutation event exists.

```json
{"schema":{"major":1,"minor":0},"event":"diagnostic","severity":"warning","tool":"ruff","rule":"F401","message":"Imported but unused","path":"src/app.py","range":{"start_byte":18,"end_byte":24},"snapshot":"initial","fixable":true}
```

Diagnostics with identical finding fields are deduplicated conservatively before resolution.
They are emitted in deterministic order once the mode has enough information to make all
required fields truthful: after convergence in check mode and after terminal apply outcomes in
default mutating mode. Initial diagnostics sort before terminal diagnostics, then order is
present paths before pathless findings; path by UTF-8
bytes; absent range before present range; start byte; end byte; severity rank
`info < warning < error`; tool; absent rule before present rule; rule; and message. BEP
arrival order and private producer identity are not observable. Fixability and resolution do
not alter ordering. One finding per event keeps lines bounded and consumers streamable.

## Notice

A `notice` represents non-fatal user-facing information that is neither a source finding
nor an operational failure.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `level` | yes | string | `info` or `warning` |
| `code` | yes | string | Stable lowercase snake-case notice code |
| `message` | yes | string | Human-readable explanation |
| `related_command` | no | string | Command that resolves the condition |
| `scope` | no | array of strings | Applicable compact main-workspace scope |
| `path` | `ignored_import` | string | Normalized workspace-relative source path containing the ignored import |
| `language` | `ignored_import` | string | Stable Gazelle source-language name used to classify the import |
| `import` | `ignored_import` | string | Exact literal dependency identity matched by the directive |

The initial stable codes are `selection_scope_mismatch`, emitted when independently selected
environment and codegen scopes differ, and `ignored_import`, emitted once per distinct source path,
language, and exact literal dependency reference accepted by `# gazelle:dx_ignore_import`. The latter omits
`scope` and `related_command`; repeated occurrences of the same import in one source file produce
one notice. Ignored-import notices sort by path, language, and import bytes. Notices do not affect
`--fail-on`, diagnostic counts, SARIF, or exit status.

```json
{"schema":{"major":1,"minor":0},"event":"notice","level":"warning","code":"ignored_import","message":"Static import intentionally contributes no Bazel dependency","path":"src/plugin.py","language":"python","import":"optional_runtime_module"}
```

## Change

Each exact workspace file change calculated by lint, typecheck, format, or generate is one
`change` event in JSON mode. A change describes intended bytes, not whether they were written.
In check mode any change makes the command fail independently of diagnostic severity. In
default mode its subsequent `mutation` outcome reports whether application succeeded.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `path` | yes | string | Normalized workspace-relative path |
| `kind` | yes | string | `modify`, or `create` for generate |
| `source_digest` | for `modify` | string | Exactly 64 lowercase hexadecimal characters encoding the BLAKE3-256 digest of the exact original bytes |
| `edits` | yes | array | Non-empty sorted exact replacement edits |
| `edits[].start_byte` | yes | unsigned integer | Inclusive byte offset in the original file |
| `edits[].end_byte` | yes | unsigned integer | Exclusive byte offset in the original file |
| `edits[].replacement` | yes | string | Exact new UTF-8 content for that range |

Edit ranges are half-open UTF-8 byte ranges against a valid UTF-8 source identified by
`source_digest`. They are in strictly increasing `start_byte` order, do not share a start
offset, and may be adjacent. Every edit satisfies `start_byte <= end_byte`; both offsets are
at most the source byte length and lie on UTF-8 code-point boundaries. For each adjacent pair,
the preceding `end_byte` is at most the next `start_byte`, so an insertion cannot sit inside a
replaced range. Empty ranges insert and empty replacement strings delete. No edit may replace
a range with identical text, and the reconstructed candidate must
differ from the original. A formatter or generator may use one edit spanning the complete
original file. A `create` change is generate-only, omits `source_digest`, requires the path to
be absent when validated, and contains exactly one `0..0` insertion whose replacement is the
complete new file. V1 quality changes only modify existing direct main-workspace non-generated sources, and no v1
workflow deletes a complete workspace file.

The event contains no original bytes. A consumer can reconstruct the candidate by validating
the digest and applying edits from the end of the original file. JSON escaping is transport
only; after JSON decoding, `replacement` is the exact proposed text. Unsupported non-UTF-8
replacement content is a protocol error rather than an alternate encoding.

Quality changes are normalized original-to-stable candidates that have passed convergence and
owner-context consensus; they are sent to the apply engine in default mode and left unapplied
in check mode. Generate changes come from the exact
Gazelle-owned result manifest. The CLI validates each existing file's current digest before
emitting its event and validates that each proposed create path is absent. It does not invoke
Git, derive structured changes from human diffs, compare
before/after workspace trees, or rerun a tool to populate events.

Quality change events and all check-mode change events are emitted by normalized path UTF-8 bytes
only after complete result-envelope or manifest validation, and before any default-mode mutation
event. Incomplete quality collection and incomplete check-mode generation emit no changes or
mutations. Default generation is the sole exception: an incomplete but structurally valid manifest
ending in a late workflow failure may report the validated attempted-prefix changes, followed by
their terminal mutation records, including confirmed writes. A malformed or contradictory manifest
fails closed and emits no change or mutation events, even if Gazelle changed the workspace before
the manifest failed validation. After the applicable validation boundary, a path-local digest,
merge, or edit failure omits that path's change and does not suppress valid unrelated paths; the
command emits the applicable operational error and fails. Check mode never emits a mutation event
and never writes a workspace file.

```json
{"schema":{"major":1,"minor":0},"event":"change","path":"src/app.py","kind":"modify","source_digest":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","edits":[{"start_byte":18,"end_byte":24,"replacement":""}]}
```

```json
{"schema":{"major":1,"minor":0},"event":"change","path":"new/package/BUILD.bazel","kind":"create","edits":[{"start_byte":0,"end_byte":0,"replacement":"py_library(\n    name = \"package\",\n)\n"}]}
```

## Mutation

Each known workspace file handled or attempted by a supported mutating workflow is one
`mutation` event.
In v1 this applies to default-mode lint, typecheck, format, and generate.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `path` | yes | string | Normalized workspace-relative path |
| `kind` | yes | string | `modify`, or `create` for generate |
| `outcome` | yes | string | `applied` or `not_applied` |
| `reason` | when not applied | string | Stable per-file or terminal operational error code |

Mutation events never duplicate source digests, edit ranges, original bytes, replacement
bytes, or generated diffs. In JSON mode, the preceding `change` event supplies the exact
attempted edits. Applied events are post-commit and omit `reason`;
`not_applied` events are post-attempt. A stale digest, edit conflict,
invalid path-local edits, or atomic-replace failure emits `not_applied` only for that path.
Valid unrelated paths may emit `applied` in the same invocation. Quality mutation events are
emitted by normalized path UTF-8 bytes after each path reaches its terminal state. Any
`not_applied` mutation makes the command fail.

For quality workflows, if collection or result-envelope validation fails before the complete
candidate path set is known, no mutation starts and no mutation event is emitted. An interruption
after mutation begins may prevent events for paths not yet attempted; preceding `applied` events
describe complete atomic file replacements and remain true.

Generate events use the same public shape but have Gazelle's native write semantics. The canonical
Gazelle integration emits exact edits and terminal outcomes in its result manifest as part of the
same execution. In default mode, validated attempted-prefix changes are emitted first; corresponding
mutation events then use deterministic Gazelle attempt order. `applied` is emitted only for a
backend-confirmed create or modification. A known attempted path that could not be written emits
`not_applied` with the applicable operational error code. If a later generation step fails, an
incomplete but structurally valid manifest may preserve the earlier validated changes and terminal
outcomes, including `applied` writes; no event claims repository-wide or Rust-managed per-file
atomicity. Check mode requires a complete manifest, emits changes rather than mutation events, and
writes nothing.

`dx` does not invoke Git, scan the workspace, parse BUILD files, rerun Gazelle, or compare
before/after trees to infer generation changes. Update emits no v1 mutation events because
its authoritative backends do not yet provide an equivalent committed-change manifest.

The private Gazelle result manifest is versioned and has a completion state. Each file record
contains normalized path, `create` or `modify`, optional original BLAKE3-256 digest under the same
rules as `change`, canonical exact edits, and, in default mode, terminal write outcome in Gazelle
attempt order. A complete check manifest describes the complete candidate set before events are
emitted. The manifest also contains normalized ignored-import records with source path, language,
and exact import string, from which text and JSON notices are rendered without scanning source
files. Duplicate audit records are deduplicated by that tuple. A default-mode late failure may end
an incomplete manifest after a structurally valid attempted prefix; after validating that prefix as
a whole, the CLI may emit its changes followed by its terminal mutation outcomes. Missing required,
malformed, contradictory, or duplicate file records fail closed: the command fails and emits no
change or mutation records from that manifest. The concrete transport between Bazel, Gazelle, and
`dx` remains an implementation decision, but it may not weaken these semantics.
The manifest is a Bazel-owned private output or equivalent private transport artifact and is
never written into the consumer source tree as generated project/integration metadata.

```json
{"schema":{"major":1,"minor":0},"event":"mutation","path":"src/app.py","kind":"modify","outcome":"applied"}
```

## Report

A `report` event confirms a successfully emitted standard report.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `format` | yes | string | `sarif`, `junit`, or `lcov`; SPDX shared-report mapping pending O58 (see below) |
| `path` | yes | string | User-selected file destination |
| `results_complete` | yes | boolean | Whether all selected result producers completed |

Only file reports produce this NDJSON event, after atomic replacement. A stdout report
cannot coexist with NDJSON; successful document output plus process status is its
confirmation in text mode.

The license family's [SPDX 2.3 JSON report](commands/audit-update-bazel.md#license-family-dx-audit-license)
is accepted direction; its shared-report format identifier and event mapping remain pending
[O58](../open-decisions.md), not a new profile defined here.

```json
{"schema":{"major":1,"minor":0},"event":"report","format":"sarif","path":"reports/lint.sarif","results_complete":true}
```

## Selection

A `selection` event confirms the successfully validated state selected by `env`, `codegen`,
or `setup`.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `setup_id` | yes | string | Lowercase BLAKE3-256 setup identity |
| `environment_id` | yes | string | Selected environment generation identity |
| `codegen_id` | yes | string | Selected codegen generation identity |

All identities are 64 lowercase hexadecimal characters. One event is emitted after any
required `.dx/setups/current` commit for every successful non-dry-run env, codegen, or
setup. An idempotent run whose requested state is already current still emits the
validated current IDs. Empty and carried-forward generations are reported exactly like
prepared generations; consumers do not infer provenance from the IDs.

```json
{"schema":{"major":1,"minor":0},"event":"selection","setup_id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","environment_id":"123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0","codegen_id":"23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01"}
```

## Operational Error

CLI, orchestration, protocol, and infrastructure failures use `error`, not `diagnostic`.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `code` | yes | string | Stable lowercase snake-case machine code |
| `message` | yes | string | Human-readable explanation without secret values |
| `path` | no | string | Relevant normalized workspace-relative path |
| `flag` | no | string | Relevant option name without its value |
| `phase` | no | string | Relevant operation phase |

Error events do not contribute to diagnostic counts or SARIF findings. They never contain
argv, option values, environment values, external labels, or raw tool output. Detailed
Bazel/tool diagnostics remain on stderr. `code` is stable machine data; `message` is for
people and is not a stable value for matching or control flow.

Initial stable codes are:

| Code | Meaning |
| --- | --- |
| `invalid_usage` | Invalid command syntax or incompatible options |
| `workspace_not_found` | No supported Bazel module root was found |
| `unsupported_workspace` | Workspace markers or configuration are unsupported |
| `invalid_scope` | Scope syntax or kind is unsupported for the command |
| `no_owner` | A file has no declared Bazel owner |
| `no_tests` | File owners map to no tests for test or coverage |
| `conflicting_option` | A forwarded option conflicts with required workflow policy |
| `bazel_unavailable` | The required Bazelisk-compatible launcher cannot run |
| `bazel_failed` | A required Bazel subprocess failed |
| `invalid_result` | A BEP artifact or result message is malformed or unsupported |
| `mutation_conflict` | Reserved compatibility code for a proposed mutation conflict without a narrower classification |
| `quality_oscillation` | A virtual quality pipeline repeated a changed source state |
| `quality_iteration_limit` | A virtual quality pipeline still changed source in its tenth and final round |
| `quality_context_disagreement` | Multiple owner/configuration pipelines produced different final bytes for one path |
| `stale_source` | Current source bytes do not match the analyzed digest |
| `managed_path_conflict` | A required managed path contains unowned state |
| `workspace_busy` | The bounded commit-lock wait expired |
| `report_failed` | A requested standard report could not be produced or committed |
| `commit_failed` | A validated source or managed-state commit failed because of filesystem or I/O error |
| `unsupported_platform` | The selected workflow has no hermetic platform support |
| `symlink_unavailable` | Required host symlink capability is unavailable |
| `internal_error` | An invariant failed without a narrower stable classification |

Adding a code is minor-compatible. Changing the meaning of an existing code is breaking.

## Command Finished

One `command_finished` event is the last event of every normally terminating JSON
invocation. Abrupt process termination such as `SIGKILL` or loss of stdout may prevent it.

| Field | Required | Type | Meaning |
| --- | --- | --- | --- |
| `exit_code` | yes | integer | Exact process exit code selected by the CLI contract |
| `results_complete` | selected commands | boolean | Whether every selected result producer returned a valid result |
| `diagnostics` | selected commands | object | Counts by `info`, `warning`, and `error` |
| `changes` | selected modes | object | Proposed-change counts by `create` and `modify` |
| `mutations` | selected modes | object | Counts by `applied` and `not_applied` |

Applicable count objects include zero values for every defined member. There is no
textual status: `exit_code == 0` means success. `results_complete=true` does not mean the
command succeeded; complete findings may still cross the configured failure threshold, or
check mode may have proposed changes.
`results_complete=false` means execution or protocol failure omitted selected results.
Commands without independently collectable results omit it.

A valid failing test case or a diagnostic crossing `--fail-on` is still a completed
result. Such findings affect `exit_code`, not `results_complete`.

`results_complete` is present exactly for lint, typecheck, format, generate, audit, test,
and coverage when they execute result-producing work. For generate it reports whether the
canonical Gazelle execution produced a complete result manifest, including edit and ignored-import
records; it does not imply
that default-mode writes were repository-wide atomic. A successfully validated empty quality
selection has `results_complete=true` because its selected producer set is complete, even
though no action was created. Dry-run and initialization failure omit it because no result
set was attempted. `diagnostics` is present exactly for executed lint, typecheck, format,
and audit, including zero values for a successful empty quality selection. `changes` is
present exactly for executed JSON-mode lint, typecheck, format, and generate in both check and
default modes, including zero values when no changes are calculated.
`mutations` is present exactly for non-dry-run default-mode lint, typecheck, format, and generate,
including when active producers return no candidate changes. A validated empty quality
selection has no mutation events or mutation count because no apply set was attempted.
Dry-run and check mode omit mutation counts.

The final event does not contain report, artifact, subprocess, tool, rule, target, or file
counts. Individual JSON events remain authoritative; compact JSON totals are a streaming-client
convenience and must equal the preceding applicable JSON events. Text and diff modes emit
no counts and carry no totals obligation.

```json
{"schema":{"major":1,"minor":0},"event":"command_finished","exit_code":1,"results_complete":true,"diagnostics":{"info":0,"warning":3,"error":1},"mutations":{"applied":0,"not_applied":2}}
```

```json
{"schema":{"major":1,"minor":0},"event":"command_finished","exit_code":1,"results_complete":true,"diagnostics":{"info":0,"warning":0,"error":0},"changes":{"create":1,"modify":2}}
```

## Event Ordering

Durable events follow actual phase order. Check-mode JSON order is:

1. Optional initialization `error` when command parsing cannot complete.
2. `command_started` when initialization succeeds.
3. `operation` events in execution order.
4. Deterministically sorted `diagnostic` events after result collection and applicable
   `notice` events when their condition is known, followed by sorted `change` events in JSON
   mode.
5. Post-commit `report` and `selection` events in normalized order.
6. Exactly one `command_finished` event.

Default mutating JSON order is:

1. Optional initialization `error`, then `command_started` and `operation` as above.
2. Sorted `change` events for the validated intended changes.
3. Post-attempt `mutation` events. Quality paths use normalized path order; generate mutations use
   deterministic Gazelle attempt order after the reportable manifest records are validated.
4. Deterministically sorted `diagnostic` events with required resolution after terminal file
   outcomes, plus applicable notices.
5. Post-commit `report` and `selection` events in normalized order.
6. Exactly one `command_finished` event.

Non-mutating commands without check mode omit changes and mutations and emit diagnostics,
notices, reports, selections, and completion after their producing phases.

An operational `error` is emitted after all durable output derived safely from the failed phase and
before `command_finished`. For partial quality collection, validated diagnostics with truthful
resolution and requested partial reports may be emitted before the error, but no change or mutation
is emitted. Incomplete check-mode generation likewise emits no change or mutation. A structurally
valid default-generation late-failure manifest may emit its validated attempted-prefix changes and
terminal mutations before the error; malformed or contradictory manifests emit neither. Per-file
apply errors are emitted after their path's `not_applied` mutation; independent file applications
may continue. No new dependent workflow phase starts after failure; report finalization over already
validated results and failure-state events are not dependent workflow execution. An initialization
error may precede `command_started` as defined above.

On the first required subprocess failure, no dependent phase or mutation starts. Already-
running independent work may settle only for safe cleanup, and its later outcome does not
replace the selected failure or reorder durable output.
The [update exception](commands/audit-update-bazel.md#dx-update) permits later independent
selected dependency sets to run after a set failure, preserving successes and reporting
blocked dependents. O12 must qualify operation boundaries, per-set reporting, and aggregate
exit selection before implementation; this does not authorize new event fields or update
mutation events, nor parallel execution.

## Dry Run

Dry-run may execute read-only Bazel `query` or `cquery` needed for scope resolution. It
emits `command_started` with `dry_run=true` and the same safe `operation` summaries as an
executing plan. It emits no `diagnostic`, `change`, `mutation`, `report`, or `selection` event
for a workflow it did not execute. Resolution errors and `command_finished` behave normally.
`--dry-run` conflicts with every `--report` request and fails before query or workflow
execution with `conflicting_option` naming `--report`.

## Standard Reports

The report formats, provisional SARIF/JUnit/LCOV profiles, pending SPDX mapping, destination validation,
deterministic ordering, and partial-document behavior are defined in
[Standard Reports](standard-reports.md). This document owns only report interaction with
live streams and the NDJSON `report` event.

## Exit Codes

`0` means success. `2` is used for CLI-detected usage, option, workspace-discovery, scope,
owner, and empty-test-mapping errors that occur before a required workflow subprocess.
`1` is used for quality-policy failure and CLI-originated execution, protocol, report, or
mutation failure. When a required subprocess exits nonzero, `dx` preserves that exact exit
code; later cleanup cannot replace it. Bazel-authoritative build, test, coverage,
generate, env, codegen, setup, and `dx bazel` therefore preserve Bazel's code when
Bazel is the failing operation. Quality commands may return `1` after a successful Bazel
invocation when normalized findings cross `--fail-on` or check mode proposes changes.
For multiple selected update sets, any failed set makes the overall command fail; exact
aggregate exit-code selection remains subject to O12 rather than the first-failure rule above.

On Unix, `dx` forwards an interrupting signal and re-raises it after safe cleanup so shell
signal semantics are preserved; no `command_finished` event is promised after signal
termination. Windows preserves the child termination result according to platform process
semantics.

## Extensible Values

Adding an event field, event kind, command name, operation phase, tool identifier, report
format, notice code, or operational error code increments the minor version and is
compatible within the same major. Consumers must tolerate unknown values in those open
sets. Severity, mutation kind, mutation outcome, and generation ID encoding are closed
sets. Change kind and edit coordinate/content semantics are also closed; adding or changing a
value requires a new major version.

## Compatibility Tests

Protocol fixtures must verify:

- O12-qualified update continuation, per-set success/failure/blocked reporting, and overall
  failure without rollback of successful independent changes or unsupported mutation events.
- Exclusive stdout ownership and arbitrary subprocess output on stderr.
- Complete deterministic unified patches in diff mode, including new files, multiple files,
  context, missing-final-newline markers, empty output, and rejected unrepresentable paths.
- Mutating diff output includes every validated intended change regardless of its later
  `applied` or `not_applied` outcome and never claims to describe final workspace state.
- One valid JSON object per NDJSON line and one schema version per invocation.
- Unknown-field and unknown-event tolerance within one major version.
- Exact field presence, omission rules, enums, stable error codes, and event ordering.
- Strict fixability and resolution presence, conservative deduplication, and mutating
  diagnostic emission only after outcomes are known.
- Deterministic diagnostics and compact scopes independent of BEP order.
- Exact change reconstruction for insertions, deletions, adjacent edits, multibyte UTF-8,
  JSON escaping, full-file replacements, and new files; replacement text is never truncated.
- Reject malformed digest encoding, invalid UTF-8 sources or replacements, unsorted or
  overlapping edits, same-offset insertions, no-op edits/candidates, duplicate file events,
  create events with a digest or existing destination, and quality-originated creates.
- Verify every check-mode change forces nonzero status, default-mode changes pair with terminal
  mutations, and aggregate counts describe deduplicated file events.
- Check/write parity: applying check-mode edits to their validated snapshot produces exactly
  the bytes submitted to or confirmed by default mode.
- No `dx`-generated argv, option values, credentials, external labels, or original source
  bytes; child-output passthrough remains explicitly untrusted. Exact proposed replacement
  text appears in JSON `change` events in both check and default modes.
- Aggregate counts matching individual events.
- Check-mode changes, default-mode mutations, and ignored-import notices deriving from equivalent
  normalized quality results or one exact Gazelle-owned result manifest, without a human diff,
  second run, source/BUILD parsing, or Git inspection.
- Incomplete quality and check-mode generation collection emits no changes or mutations; a
  structurally valid default-generation late-failure manifest reports only its validated attempted
  prefix, while malformed or contradictory manifests fail closed.
- Atomic file reports, exclusive stdout reports, and conformance with
  [Standard Reports](standard-reports.md).
- Equivalent text, diff, NDJSON, and standard-report semantics over one normalized result set.
