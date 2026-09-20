#!/usr/bin/env bash
# Structured-cohort qualification harness (issue #419).
#
# Qualifies the as-built structured quality-cohort record with fixture
# evidence and owned gaps, without claiming Supported and without a false
# adapter claim:
# - delivered: decided checksummed native/self-contained artifact route for
#   `buf` (self-contained per-platform binaries with published checksums, no
#   target compiler context unlike clang-tidy, execution-platform lazy) plus
#   decided authoritative-toolchain route for qmlformat/qmllint from the Qt
#   distribution (Qt-last ordering decided; exact Qt distribution identity,
#   licensing, and platform artifact qualification remain pending), initial
#   artifact research rows as observations for digests (versions qualified
#   seed-only under issue #488), adapter-input notes (buf
#   `--error-format=json` JSONL as the faithful shape with no SARIF in
#   1.71.0, `STANDARD` rule selection plus module-root-sensitive
#   `PACKAGE_DIRECTORY_MATCH` plus `--path` scoping; qmlformat stdout plus
#   `-i` with `.qmlformat.ini` upward settings; qmllint `--json` with
#   `.qmllint.ini` plus `//qmllint enable/disable`; whole-file rewrite versus
#   check-only fix modes with the provisional sandbox-apply-and-diff flow,
#   never silently dropped), native-config defaults qualified seed-only
#   under issue #488 (`buf` `STANDARD` as the upstream built-in default lint
#   set, qmlformat/qmllint ini discovery as native interpretation, no
#   auto-supplied preset), parity-deferred protobuf/qml with owner plus frozen
#   route, classification-only taxonomy with no curated defaults and no
#   native-config binding;
# - open under #419 with honest records: exact artifact digests
#   plus Qt distribution qualification, parser plus runner-matrix pass/fail
#   plus fix/format evidence per adapter-backed class, native-config
#   qualification against the native-config contract, platform plus
#   consumer plus release evidence. REAL_ADAPTERS claims protobuf/qml only
#   when green.
#
# Versioned here, run by CI via `bazel run //tools/ci:structured_cohort_qualification`,
# following //tools/ci:native_cohort_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
aspects="quality/real_aspects.bzl"

# No false adapter claim for the structured cohort: none of the cohort tool
# IDs appear in REAL_ADAPTERS. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not.
cohort_claim=""
for tool in buf qmlformat qmllint; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    cohort_claim="$cohort_claim $tool:claimed"
  fi
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "false adapter claim for structured cohort:$cohort_claim"
fi

# Parity deferrals own protobuf/qml with owner plus frozen route plus the
# #419 live-successor record (closed #307 owns nothing here).
if grep -q -F -e '"protobuf": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"qml": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'checksummed standalone artifact (buf format+lint)' "$parity" &&
  grep -q -F -e 'authoritative Qt distribution toolchain (qmlformat, qmllint)' "$parity" &&
  grep -q -F -e 'issue #419' "$parity"; then
  ok
else
  bad "parity deferrals lost the structured protobuf/qml owner plus frozen route plus #419 record"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"protobuf": "protobuf"' "$adapters" &&
  grep -q -F -e '"qml": "qml"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the protobuf/qml classification"
fi

# Classification-only today: protobuf/qml families carry no curated defaults
# (curated membership unchanged; no native default selected).
cohort_curated=""
for family in '"protobuf": {' '"qml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim a structured family before adapters land:$cohort_curated"
fi

# No hidden structured native-config preset: no cohort binding exists in the
# typed native-config rules (adapters run pinned upstream defaults until
# #419 qualifies checked-in policy against the native-config contract; the
# provisional buf STANDARD plus qmlformat/qmllint ini suggestions stay review
# inputs, never supplied configs).
cohort_config=""
for tool in buf qmlformat qmllint qml; do
  if grep -q -F -e "${tool}_config" "$native"; then
    cohort_config="$cohort_config $tool:preset"
  fi
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config carries a hidden structured preset:$cohort_config"
fi

# No false green claim: the runner matrix carries no structured cells yet, so
# REAL_ADAPTERS cannot claim protobuf/qml (claims land only with green
# pass/fail plus fix/format evidence per adapter-backed class).
if ! grep -q -F -e 'matrix_protobuf_' "$matrix" &&
  ! grep -q -F -e 'matrix_qml_' "$matrix"; then
  ok
else
  bad "runner matrix claims a structured cell without adapter qualification"
fi

# Tool acquisition keeps the decided checksummed buf route with no
# target-compiler context plus execution-platform laziness and no false
# claim, owned by #419 (live successor to closed #307 for the protobuf class).
if grep -q -F -e 'Decided route: `buf` takes the checksummed' "$acquisition" &&
  grep -q -F -e 'needs no target compiler context' "$acquisition" &&
  grep -q -F -e 'execution-platform lazy' "$acquisition" &&
  grep -q -F -e 'no adapter claims `protobuf` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #419)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided buf route, no-compiler-context plus laziness honesty, or #419 ownership or no-claim honesty"
fi

# Tool acquisition keeps the decided Qt-last authoritative-toolchain route for
# qmlformat/qmllint with distribution identity plus licensing plus platform
# artifacts pending and no false claim, owned by #419.
if grep -q -F -e 'Decided route (Qt last)' "$acquisition" &&
  grep -q -F -e 'qmlformat and qmllint take the authoritative-toolchain route' "$acquisition" &&
  grep -q -F -e 'exact Qt distribution identity, licensing, and platform' "$acquisition" &&
  grep -q -F -e 'no adapter claiming `qml` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #419)' "$acquisition" &&
  grep -q -F -e 'Qt closed that order' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Qt route, distribution-identity honesty, Qt-last ordering, or #419 ownership or no-claim honesty"
fi

# Tool acquisition keeps initial artifact research rows for the cohort as
# observations, not pins, with byte-identity risk explicit.
cohort_research=""
for tool in '| buf |' '| qmlformat |' '| qmllint |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'owned by issue #419' "$acquisition" &&
  grep -q -F -e 'observations, not pins' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a structured research row or its observations-not-pins honesty:$cohort_research"
fi

# Tool integrations keep the structured adapter-input notes:
# buf JSONL as the faithful shape (no SARIF) with STANDARD plus
# module-root-sensitive scoping recorded not silent, qmlformat stdout plus
# `-i` with ini settings, qmllint `--json` with ini plus comment scoping,
# whole-file rewrite versus check-only fix modes, Qt-last ordering,
# versions qualified under #488 with digests as observations, no adapter claim.
if grep -q -F -e '**Structured cohort (issue #419' "$integrations" &&
  grep -q -F -e 'no adapter claims `protobuf` or `qml` yet' "$integrations" &&
  grep -q -F -e 'no SARIF in 1.71.0' "$integrations" &&
  grep -q -F -e 'PACKAGE_DIRECTORY_MATCH' "$integrations" &&
  grep -q -F -e 'not silently dropped' "$integrations" &&
  grep -q -F -e '.qmlformat.ini' "$integrations" &&
  grep -q -F -e '--json <file>' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$integrations" &&
  grep -q -F -e 'observations, not pins' "$integrations"; then
  ok
else
  bad "tool-integrations lost its structured adapter-input notes or #488 versions honesty"
fi

# Support matrix keeps the structured routes plus qualified native-config
# defaults (issue #488) plus adapter-input notes plus cohort tracking, all
# citing #419 for adapters/digests without approving hidden presets or
# claiming support.
if grep -q -F -e 'checksummed native/self-contained artifact route (issue #419' "$support" &&
  grep -q -F -e 'authoritative-toolchain route from the Qt distribution (issue #419' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$support" &&
  grep -q -F -e 'STANDARD' "$support" &&
  grep -q -F -e 'upstream built-in default' "$support" &&
  grep -q -F -e 'owned by' "$support" &&
  grep -q -F -e 'issue #419' "$support" &&
  grep -q -F -e 'itemized under issue #419' "$support" &&
  grep -q -F -e 'stay open' "$support" &&
  grep -q -F -e '(issue #419)' "$support" &&
  grep -q -F -e 'to issue #419.' "$support"; then
  ok
else
  bad "support-matrix lost its structured routes, qualified defaults, adapter notes, or #419 cohort tracking"
fi

# Tool baseline keeps the Protocol Buffer/QML coverage rows (integration inventory,
# not a support claim).
if grep -q -F -e '| Protocol Buffer | buf format | buf lint |' "$baseline" &&
  grep -q -F -e '| QML | qmlformat | qmllint |' "$baseline"; then
  ok
else
  bad "tool-baseline lost its Protocol Buffer/QML coverage rows"
fi

# Curated posture unchanged: naming an integration does not enable it by
# default; enabled tools use pinned native defaults unless checked-in
# native config supplies policy (no hidden preset authorized here).
if grep -q -F -e 'Naming a tool in the matrix does not enable it by default' "$baseline" &&
  grep -q -F -e 'Tool selection does not' "$baseline" &&
  grep -q -F -e 'defines no hidden rule' "$native_doc"; then
  ok
else
  bad "curated-default posture lost its no-hidden-preset honesty"
fi

# Functional: parity gate shape still fails closed on drift.
if grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" &&
  grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" &&
  grep -q -F -e 'no adapter class is unclassified' "$parity" &&
  grep -q -F -e 'no classified class lacks a disposition' "$parity"; then
  ok
else
  bad "parity gate lost its fail-closed shape"
fi

dx_test_summary "Structured-cohort qualification harness"
