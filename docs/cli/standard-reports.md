# Standard Reports

## Contract

The durable report surface follows [ADR 0006](../decisions/0006-cli-command-surface.md).
Exact profiles and partial-document rules below remain provisional until M06/M10 qualification.

Commands accept repeatable `--report <format>=<destination>`, where destination is `-`, a
workspace-relative path, or an absolute path. Parent directories must already exist.
Unsupported formats, duplicate format/destination pairs, and multiple stdout reports fail
before Bazel execution. File reports are staged and atomically replaced only after the
document validates.

The live-stream interaction, `report` NDJSON event, exit codes, and partial-result state
are defined in [Output Protocol](output-protocol.md). `--dry-run` conflicts with every
report request.

The provisional initial mapping is:

| Commands | Format | Contract |
| --- | --- | --- |
| `lint`, `typecheck`, `audit` | SARIF 2.1.0 | Normalized findings |
| `test` | JUnit XML | Bazel-owned test cases and infrastructure failures |
| `coverage` | LCOV | Validated Bazel combined tracefile |

The license family's [SPDX 2.3 JSON report](commands/audit-update-bazel.md#license-family-dx-audit-license)
is accepted direction. Its shared `--report` mapping and profile remain pending
[O58](../open-decisions.md); this table is not an exhaustive prohibition of that report.

`build`, `format`, `update`, `generate`, `codegen`, `env`, and `setup` have no initial
standard report. `dx bazel` uses native Bazel options for BEP or other Bazel-owned output.
New formats require an established standard or authoritative upstream output.

Multiple file report events are emitted bytewise by format and then normalized destination
path, independent of option order. Report generation may preserve user option order
internally when required, but durable events remain deterministic.

## SARIF 2.1.0

SARIF uses one deterministically ordered run per tool adapter because each SARIF run has
one tool driver. Runs use stable tool/rule IDs and artifact URIs relative to the workspace
when available. Severity maps to SARIF `note`, `warning`, and `error`. Required line/
column regions derive from the same source snapshot as canonical byte ranges.

SARIF results represent findings in the actual workspace bytes after the command for scanner
interoperability. Check-mode lint and typecheck reports include initial diagnostics from the
unchanged analyzed snapshot, including guaranteed-fixable findings. Default mutating reports
use terminal diagnostics for candidates that applied and initial diagnostics for files left
unchanged; they omit initial findings resolved by an applied candidate. A tool run remains
present with an empty `results` array when every finding was fixed. This follows common scanner
behavior, including GitHub code scanning's treatment of uploaded results as current alerts,
while NDJSON remains the complete initial/terminal same-invocation audit trail.

V1 does not emit `result.baselineState` for same-run mutation resolution. SARIF baseline
states describe a comprehensive comparison with a separate baseline run; `absent` is not an
alias for a finding fixed during the current invocation. Stable rule IDs, paths, and
fingerprints are retained across reports so a result-management system can match subsequent
complete runs normally.

V1 does not emit SARIF `fixes`. SARIF fixes belong to one result and should contain the
minimal replacements that resolve that result. The v1 internal protocol guarantees whether a
diagnostic is resolved by the complete same-file candidate but does not expose a minimal
diagnostic-specific edit subset. File-level exact edits remain available through NDJSON
`change` events and diff output. A future association may map guaranteed result-specific edits
to SARIF `fixes` without changing the current-result policy.

When collection is partial, SARIF retains validated findings and records an unsuccessful
invocation in affected tool runs. A failure not attributable to an emitted tool run adds
one final `rules_dx` infrastructure run with `executionSuccessful=false` and one
`toolExecutionNotification` carrying the stable operational error code. The process
remains failed.

Partial SARIF is marked for archival and debugging but must not be uploaded as an authoritative
replacement scan: omitted results could be interpreted as resolved by result-management
systems. CI integrations gate authoritative upload on `results_complete=true`.

## JUnit XML

JUnit derives from Bazel-reported test XML artifacts, not console scraping. The output is
UTF-8 XML with one `<testsuites name="dx">` root. Each Bazel test target contributes one
`<testsuite name="//pkg:target">`; cases preserve Bazel-provided names, durations,
`<failure>`, `<error>`, `<skipped>`, `<system-out>`, and `<system-err>` content after safe
XML parsing.

Suite and case order is bytewise by target label, case name, shard index, and attempt
index. Root and suite `tests`, `failures`, `errors`, `skipped`, and `time` attributes are
recomputed. Retries or shards reported as separate Bazel attempts remain separate cases.
When either index is nonzero, case names append
` [shard=<zero-based-shard>,attempt=<zero-based-attempt>]`; an existing identical display
name is disambiguated by those same indices rather than encounter order.

Unsupported or malformed XML makes collection partial. A partial invocation retains
completed suites and adds one suite named `dx.infrastructure` with one error case named
`incomplete_results`, reflected in aggregate counts. The process remains failed.

## LCOV

Coverage requests Bazel's authoritative combined LCOV report. `dx` validates that the BEP-
reported output is a syntactically complete LCOV tracefile and writes or streams those
exact bytes; it does not merge counters or invent records.

If Bazel reports a valid combined tracefile despite a failed invocation, it may be emitted
as partial. LCOV has no portable completeness field, so
`command_finished.results_complete=false`, or a stderr diagnostic in stdout-report mode,
marks it partial. If no valid combined tracefile is reported, no LCOV report is emitted.

## Partial And Failed Reports

Validated results remain useful when another keep-going action fails. `dx` emits a valid
marked partial report and retains the nonzero command exit code. If no valid standard
document can be constructed, no report is emitted. A partial file report atomically
replaces its destination so stale complete data is never mistaken for the current run.
Authoritative upload remains gated on `results_complete=true` per the SARIF section above;
that gate is owned by CI integrations, not by the report producer.

A failure to validate, write, or commit a requested report emits `report_failed` and makes
an otherwise successful invocation fail. It does not alter the underlying workflow's
findings or mutation plan.

## GitHub Reporting Integration

See [Consumer GitHub CI](../github-ci.md) for the execution and reporting contract.

### Consumer Setup

See [Consumer Setup](../github-ci.md#consumer-setup) for the reusable workflow setup contract.

## Compatibility Tests

Golden and schema fixtures verify:

- SARIF version, per-tool runs, stable rules, byte-to-line conversion, and partial runs.
- JUnit root/suite shape, counts, attempt/shard identity, logs, ordering, and partial cases.
- Exact-byte LCOV passthrough from one BEP-reported combined tracefile.
- Atomic file destinations and exclusive stdout documents.
- Deterministic output independent of BEP and report-option order.
- No valid-result failure emits no report, while valid partial results replace stale files.
