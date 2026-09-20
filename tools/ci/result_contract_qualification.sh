#!/usr/bin/env bash
# Quality core plus result contract qualification harness (issue #511).
#
# Qualifies the quality core plus result contract mappings with fixture
# evidence pinned in `quality/tests/fixtures/result_contract/pins.bzl`
# (plus `result_contract.expected`), without claiming qualified
# platforms, qualified consumers, qualified releases, or Supported:
# - severity mapping: proto INFO plus WARNING plus ERROR map to NDJSON
#   info plus warning plus error with shared rank, UNSPECIFIED rejected.
# - capability mapping: LINT plus TYPECHECK plus FORMAT plus AUDIT with
#   UNSPECIFIED rejected.
# - diagnostic mapping: tool plus message plus rule plus path plus range
#   plus fixable plus snapshot map with path and range presence rules.
# - path mapping: normalized workspace-relative paths mirror dx_path
#   classify in both validators.
# - digest mapping: BLAKE3-256 32 raw bytes in proto map to 64 lowercase
#   hex in source_digest; mismatch fails as stale_source.
# - change mapping: FileEdits map to modify changes with exact UTF-8
#   replacements; create stays generate-only.
# - edit ordering: sorted non-overlapping distinct starts; adjacent
#   valid; same-offset plus insertion-inside plus no-op plus invalid
#   UTF-8 rejected.
# - fixability mapping: true only with exact same-file candidate plus
#   absent terminal; terminal always false; conservative dedup; never
#   inferred.
# - resolution mapping: check omits; default initials require fixed
#   plus remaining plus not_applied; terminal omits.
# - stability mapping: STABLE plus OSCILLATION plus ITERATION_LIMIT with
#   UNSPECIFIED rejected; at most ten rounds; only STABLE carries
#   replacements.
# - threshold parity: info plus warning plus error with never rejected;
#   evaluator and CLI share rank; replacements plus non-stable plus
#   fixable fail exactly as check mode.
# - collection mapping: dx_results via BEP named sets with
#   decode_validated per artifact; incomplete emits no change plus no
#   mutation; deterministic order independent of BEP; no aggregate.
# - versioning mapping: unknown major rejected; newer minors accepted
#   when bytes satisfy rules; unknown fields ignored; deterministic
#   bytes; semantic compare.
# - emission mapping: check emits change with no mutation; default emits
#   change before mutation; any check change fails independently of
#   severity.
# - rejected: bare contract with no owned mappings.
# - owned with honest records: SPDX parsing plus emission plus update
#   aggregate stay owned under issue #511; platform plus consumer plus
#   release evidence stays owned gap; backends provisional; no Supported
#   claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:result_contract_qualification`,
# following //tools/ci:remediation_bounds_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="quality/tests/fixtures/result_contract/pins.bzl"
pins_build="quality/tests/fixtures/result_contract/BUILD.bazel"
expected="quality/tests/fixtures/result_contract/result_contract.expected"
proto="quality/result.proto"
result_lib="quality/result/src/lib.rs"
evaluator_lib="quality/evaluator/src/lib.rs"
evaluator_main="quality/evaluator/src/main.rs"
results="cli/cli/src/exec/results.rs"
plan="cli/cli/src/plan.rs"
findings="cli/output/src/findings.rs"
changes="cli/output/src/changes.rs"
severity="cli/output/src/severity.rs"
validation="cli/output/src/validation.rs"
matrix="docs/product/support-matrix.md"
proto_doc="docs/quality/quality-result-protocol.md"
output_doc="docs/cli/output-protocol.md"
reports_doc="docs/cli/standard-reports.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present (issue #511).
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "result contract fixture missing (want $pins plus $pins_build plus result_contract.expected)"
fi

# Pins record the severity plus capability mappings with shared rank.
if grep -q -F -e 'proto INFO maps to NDJSON info' "$pins" &&
  grep -q -F -e 'proto WARNING maps to NDJSON warning' "$pins" &&
  grep -q -F -e 'proto ERROR maps to NDJSON error' "$pins" &&
  grep -q -F -e 'SEVERITY_UNSPECIFIED rejected' "$pins" &&
  grep -q -F -e 'evaluator rank mirrors CLI meets_threshold rank' "$pins" &&
  grep -q -F -e 'CAPABILITY LINT maps to lint pipeline' "$pins" &&
  grep -q -F -e 'CAPABILITY TYPECHECK maps to typecheck pipeline' "$pins" &&
  grep -q -F -e 'CAPABILITY FORMAT maps to format pipeline' "$pins" &&
  grep -q -F -e 'CAPABILITY AUDIT maps to audit action' "$pins" &&
  grep -q -F -e 'CAPABILITY_UNSPECIFIED rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its severity plus capability mappings with shared rank under issue #511"
fi

# Pins record the diagnostic plus path mappings with presence rules.
if grep -q -F -e 'tool_id maps to tool' "$pins" &&
  grep -q -F -e 'message maps to message' "$pins" &&
  grep -q -F -e 'rule_id maps to rule' "$pins" &&
  grep -q -F -e 'path maps to path' "$pins" &&
  grep -q -F -e 'start_byte plus end_byte map to range' "$pins" &&
  grep -q -F -e 'fixable maps to fixable' "$pins" &&
  grep -q -F -e 'snapshot maps to initial plus terminal' "$pins" &&
  grep -q -F -e 'normalized slash-separated workspace-relative path' "$pins" &&
  grep -q -F -e 'absolute paths rejected' "$pins" &&
  grep -q -F -e 'backslash paths rejected' "$pins" &&
  grep -q -F -e 'dot components rejected' "$pins" &&
  grep -q -F -e 'dotdot components rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its diagnostic plus path mappings under issue #511"
fi

# Pins record the digest plus change plus edit mappings.
if grep -q -F -e 'BLAKE3-256 over exact file bytes' "$pins" &&
  grep -q -F -e 'exactly 32 raw bytes in proto' "$pins" &&
  grep -q -F -e 'exactly 64 lowercase hex in source_digest' "$pins" &&
  grep -q -F -e 'digest mismatch fails as stale_source' "$pins" &&
  grep -q -F -e 'FileEdits path maps to path' "$pins" &&
  grep -q -F -e 'original_digest maps to source_digest' "$pins" &&
  grep -q -F -e 'edits map to edits with exact UTF-8 replacement' "$pins" &&
  grep -q -F -e 'quality kind is modify' "$pins" &&
  grep -q -F -e 'create stays generate-only' "$pins" &&
  grep -q -F -e 'strictly start-byte-sorted non-overlapping edits' "$pins" &&
  grep -q -F -e 'adjacent edits valid' "$pins" &&
  grep -q -F -e 'same-offset edits rejected' "$pins" &&
  grep -q -F -e 'insertion inside replaced range rejected' "$pins" &&
  grep -q -F -e 'no-op edits plus candidates rejected' "$pins" &&
  grep -q -F -e 'invalid UTF-8 source plus replacement rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its digest plus change plus edit mappings under issue #511"
fi

# Pins record the fixability plus resolution mappings.
if grep -q -F -e 'fixable true only with exact same-file candidate plus absent terminal' "$pins" &&
  grep -q -F -e 'terminal diagnostics always fixable false' "$pins" &&
  grep -q -F -e 'conservative duplicate merging with one false makes false' "$pins" &&
  grep -q -F -e 'fixability never inferred from another change' "$pins" &&
  grep -q -F -e 'check mode omits resolution' "$pins" &&
  grep -q -F -e 'fixed means absent terminal plus applied candidate' "$pins" &&
  grep -q -F -e 'remaining means present in terminal snapshot' "$pins" &&
  grep -q -F -e 'not_applied means absent terminal plus unapplied candidate' "$pins" &&
  grep -q -F -e 'terminal diagnostics omit resolution' "$pins"; then
  ok
else
  bad "pins.bzl lost its fixability plus resolution mappings under issue #511"
fi

# Pins record the convergence plus threshold parity mappings.
if grep -q -F -e 'STABLE may carry replacements' "$pins" &&
  grep -q -F -e 'OSCILLATION completes with no replacement' "$pins" &&
  grep -q -F -e 'ITERATION_LIMIT completes with no replacement' "$pins" &&
  grep -q -F -e 'CONVERGENCE_UNSPECIFIED rejected' "$pins" &&
  grep -q -F -e 'at most ten complete cross-tool rounds' "$pins" &&
  grep -q -F -e 'fail_on info maps to Threshold Info' "$pins" &&
  grep -q -F -e 'fail_on warning maps to Threshold Warning' "$pins" &&
  grep -q -F -e 'fail_on error maps to Threshold Error' "$pins" &&
  grep -q -F -e 'fail_on never rejected' "$pins" &&
  grep -q -F -e 'replacements fail at every threshold' "$pins" &&
  grep -q -F -e 'non-stable convergence fails' "$pins" &&
  grep -q -F -e 'fixable still fails without apply' "$pins"; then
  ok
else
  bad "pins.bzl lost its convergence plus threshold parity mappings under issue #511"
fi

# Pins record the collection plus versioning plus emission plus rejected plus owned gaps.
if grep -q -F -e 'dx_results output group via BEP named sets' "$pins" &&
  grep -q -F -e 'decode_validated per artifact' "$pins" &&
  grep -q -F -e 'incomplete collection emits no change plus no mutation' "$pins" &&
  grep -q -F -e 'deterministic order independent of BEP order' "$pins" &&
  grep -q -F -e 'no invocation-level aggregate action' "$pins" &&
  grep -q -F -e 'unknown major rejected' "$pins" &&
  grep -q -F -e 'newer minors accepted when bytes satisfy rules' "$pins" &&
  grep -q -F -e 'unknown fields ignored' "$pins" &&
  grep -q -F -e 'deterministic bytes for identical inputs' "$pins" &&
  grep -q -F -e 'semantic compare not envelope equality' "$pins" &&
  grep -q -F -e 'check mode emits change with no mutation' "$pins" &&
  grep -q -F -e 'default emits change before terminal mutation' "$pins" &&
  grep -q -F -e 'any check-mode change fails independently of severity' "$pins" &&
  grep -q -F -e '"bare contract"' "$pins" &&
  grep -q -F -e 'SPDX parsing plus policy-table loading stays owned under issue #511' "$pins" &&
  grep -q -F -e 'live SPDX emission stays owned under issue #511' "$pins" &&
  grep -q -F -e 'update aggregate exit codes stay owned under issue #511' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its collection plus versioning plus emission plus rejected plus owned gaps under issue #511"
fi

# Proto stays the frozen source of truth with fixed numbers plus reserved ranges.
if grep -q -F -e 'enum Severity' "$proto" &&
  grep -q -F -e 'SEVERITY_UNSPECIFIED = 0' "$proto" &&
  grep -q -F -e 'enum Capability' "$proto" &&
  grep -q -F -e 'CAPABILITY_UNSPECIFIED = 0' "$proto" &&
  grep -q -F -e 'enum Convergence' "$proto" &&
  grep -q -F -e 'CONVERGENCE_UNSPECIFIED = 0' "$proto" &&
  grep -q -F -e 'message Diagnostic' "$proto" &&
  grep -q -F -e 'message FileEdits' "$proto" &&
  grep -q -F -e 'message QualityResult' "$proto" &&
  grep -q -F -e 'reserved 4 to 15' "$proto" &&
  grep -q -F -e 'reserved 13 to 24' "$proto"; then
  ok
else
  bad "quality/result.proto lost its frozen severity plus capability plus convergence plus diagnostic plus edits plus result shape"
fi

# Result crate keeps the validation plus codec plus digest plus round bound.
if grep -q -F -e 'pub fn validate' "$result_lib" &&
  grep -q -F -e 'pub fn encode_validated' "$result_lib" &&
  grep -q -F -e 'pub fn decode_validated' "$result_lib" &&
  grep -q -F -e 'MAX_COMPLETED_ROUNDS' "$result_lib" &&
  grep -q -F -e 'DIGEST_LEN' "$result_lib" &&
  grep -q -F -e 'UnsupportedMajor' "$result_lib" &&
  grep -q -F -e 'ReplacementsWithoutStability' "$result_lib" &&
  grep -q -F -e 'dx_digest::blake3' "$result_lib"; then
  ok
else
  bad "quality/result/src/lib.rs lost its validate plus codec plus digest plus round-bound contract"
fi

# Evaluator keeps the threshold parity plus replacement plus stability plus fixability rules.
if grep -q -F -e 'pub enum Threshold' "$evaluator_lib" &&
  grep -q -F -e 'pub fn parse_threshold' "$evaluator_lib" &&
  grep -q -F -e 'pub fn evaluate' "$evaluator_lib" &&
  grep -q -F -e 'proposes {} replacement file(s), independently of severity' "$evaluator_lib" &&
  grep -q -F -e 'convergence is {}, want STABLE' "$evaluator_lib" &&
  grep -q -F -e 'fixable' "$evaluator_lib" &&
  grep -q -F -e 'decode_validated' "$evaluator_main" &&
  grep -q -F -e 'unknown fail_on' "$evaluator_lib"; then
  ok
else
  bad "quality/evaluator lost its threshold plus replacement plus stability plus fixability parity"
fi

# CLI collection keeps the proto-to-event mappings over dx_results.
if grep -q -F -e 'pub(crate) fn map_severity' "$results" &&
  grep -q -F -e 'pub(crate) fn map_diagnostic' "$results" &&
  grep -q -F -e 'pub(crate) fn map_change' "$results" &&
  grep -q -F -e 'pub(crate) fn collect_results' "$results" &&
  grep -q -F -e 'decode_validated' "$results" &&
  grep -q -F -e 'OUTPUT_GROUP' "$results" &&
  grep -q -F -e 'dx_results' "$plan"; then
  ok
else
  bad "cli collection lost its map_severity plus map_diagnostic plus map_change plus dx_results wiring"
fi

# CLI output keeps the NDJSON event plus ordering plus digest plus edit validation.
if grep -q -F -e 'pub fn diagnostic_event' "$findings" &&
  grep -q -F -e 'pub fn sort_diagnostics' "$findings" &&
  grep -q -F -e 'pub fn change_event' "$changes" &&
  grep -q -F -e 'pub fn mutation_event' "$changes" &&
  grep -q -F -e 'pub fn meets_threshold' "$severity" &&
  grep -q -F -e 'pub fn parse_digest' "$validation" &&
  grep -q -F -e 'pub fn check_edits' "$validation" &&
  grep -q -F -e 'pub fn check_path' "$validation"; then
  ok
else
  bad "cli/output lost its diagnostic plus change plus mutation plus threshold plus digest plus edit mappings"
fi

# Support matrix owns the qualified core plus result contract record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #511' "$matrix" &&
  grep -q -F -e 'result_contract_qualification' "$matrix" &&
  grep -q -F -e 'quality/tests/fixtures/result_contract/pins.bzl' "$matrix" &&
  grep -q -F -e 'bare contract rejected' "$matrix" &&
  ! grep -q -E -e '^\| .* \| Supported' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified result contract record with bare-contract rejection under issue #511"
fi

# Protocol docs own the qualified record; SPDX plus update aggregate stay owned gaps.
if grep -q -F -e 'qualified seed-only under issue #511' "$proto_doc" &&
  grep -q -F -e 'result_contract_qualification' "$proto_doc" &&
  grep -q -F -e 'quality/tests/fixtures/result_contract/pins.bzl' "$proto_doc" &&
  grep -q -F -e 'bare contract rejected' "$proto_doc" &&
  grep -q -F -e 'open work under issue #511' "$output_doc" &&
  grep -q -F -e 'open work under issue #511' "$reports_doc"; then
  ok
else
  bad "protocol docs lost their qualified result contract record plus SPDX plus update owned gaps under issue #511"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "result_contract_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:result_contract_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the result_contract_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture expected covers the full mapping with owned gaps.
if grep -q -F -e 'qualified seed-only under issue #511' "$expected" &&
  grep -q -F -e 'bare contract rejected' "$expected" &&
  grep -q -F -e 'BLAKE3-256' "$expected" &&
  grep -q -F -e 'conservative dedup' "$expected" &&
  grep -q -F -e 'dx_results' "$expected" &&
  grep -q -F -e 'owned gap' "$expected"; then
  ok
else
  bad "result_contract.expected lost mapping coverage (want qualified plus bare-rejected plus digest plus dedup plus dx_results plus owned gap, issue #511)"
fi

# Live proof: result plus evaluator plus output suites plus the fixture build stay green.
if bazel test //quality/result:quality_result_test //quality/evaluator:quality_evaluator_test //cli/output:dx_output_test --noshow_progress >/dev/null 2>&1 &&
  bazel build //quality/tests/fixtures/result_contract/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "result contract live shapes failed (want quality_result plus evaluator plus dx_output plus fixture green, issue #511)"
fi

dx_test_summary "result contract qualification harness"
