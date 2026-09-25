#!/usr/bin/env bash
# File-family quality qualification harness.
#
# Qualifies the as-built file-family quality record with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered: provider-class applicability verification (never inferred
#   from suffix), Starlark/Buildifier plus TOML/Taplo adapter-backed
#   evidence (matrix pass/fail, parser, curated defaults, native-config),
#   parity-deferred routes with owner plus frozen acquisition per family
#   (CSS Prettier/Stylelint, HTML templates djlint, Protobuf buf,
#   YAML yamlfmt/yamllint, text keep-sorted, Shell shfmt/ShellCheck,
#   CUE/Jsonnet/Pkl/QML/Terraform feasibility), tool-baseline plus
#   acquisition routing, registry singularity;
# - open under with honest records: adapter execution for the
#   deferred families, platform plus consumer plus release evidence,
#   exact pins/digests/rule-sets/adapter mappings per family.
#
# Versioned here, run by CI via `bazel run //tools/ci:file_family_qualification`,
# following //tools/ci:consumer_ci_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
sources_doc="docs/quality/quality-sources.md"
integrations="docs/quality/tool-integrations.md"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
applicability="quality/applicability.bzl"
sources="quality/sources.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
matrix_data="quality/testdata/runner_matrix_data.bzl"
subjects="quality/testdata/BUILD.bazel"
aspects="quality/real_aspects.bzl"

# Support matrix owns the Quality And File Families section with the
# qualified seed-only record under.
# Tool baseline owns the matching file-family rows.
if grep -q -F -e '| CSS, Less, SCSS | Prettier | Stylelint |' "$baseline" &&
  grep -q -F -e '| HTML templates | djlint |' "$baseline" &&
  grep -q -F -e '| Protocol Buffer | buf format | buf lint |' "$baseline" &&
  grep -q -F -e '| Starlark | Buildifier | Buildifier |' "$baseline" &&
  grep -q -F -e '| TOML | Taplo | Taplo |' "$baseline" &&
  grep -q -F -e '| YAML | yamlfmt | yamllint |' "$baseline" &&
  grep -q -F -e 'keep-sorted' "$baseline" &&
  grep -q -F -e '| Shell | shfmt | ShellCheck |' "$baseline"; then
  ok
else
  bad "tool-baseline lost a file-family row (css/djlint/buf/buildifier/taplo/yaml/keep-sorted/shell)"
fi

# Tool baseline keeps the feasibility tool names for cue/jsonnet/pkl/qml/terraform.
if grep -q -F -e '| CUE | cue fmt |' "$baseline" &&
  grep -q -F -e '| Jsonnet | jsonnetfmt |' "$baseline" &&
  grep -q -F -e '| Pkl | pkl |' "$baseline" &&
  grep -q -F -e '| QML | qmlformat | qmllint |' "$baseline" &&
  grep -q -F -e '| Terraform | terraform fmt |' "$baseline"; then
  ok
else
  bad "tool-baseline lost a feasibility row (cue/jsonnet/pkl/qml/terraform)"
fi

# Tool acquisition routes every file-family tool to a frozen delivery class.
if grep -q -F -e 'Private pure-JavaScript graph plus shared managed Node' "$acquisition" &&
  grep -q -F -e 'Stylelint' "$acquisition" &&
  grep -q -F -e 'Private wheel-only Python graph plus shared managed Python' "$acquisition" &&
  grep -q -F -e 'djlint' "$acquisition" &&
  grep -q -F -e 'Checksummed native/self-contained artifact' "$acquisition" &&
  grep -q -F -e 'keep-sorted' "$acquisition" &&
  grep -q -F -e 'shfmt' "$acquisition" &&
  grep -q -F -e 'yamlfmt' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a file-family delivery route (node/python/standalone)"
fi

# Tool integrations keep the decided buf route, delivered under #799.
if grep -q -F -e 'native/self-contained artifact route for `buf` (self-contained per-platform' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #799' "$integrations" &&
  grep -q -F -e 'cohort delivered under #799' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its delivered buf route under #799"
fi

# Tool integrations keep the qml authoritative-toolchain route, delivered under #799.
if grep -q -F -e 'authoritative-toolchain route for qmlformat/qmllint from the Qt distribution' "$integrations" &&
  grep -q -F -e 'Qt-last ordering decided' "$integrations" &&
  grep -q -F -e 'cohort delivered under #799' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its delivered qml route under #799"
fi

# Parity no longer defers the delivered file-family classes (all delivered
# under #800; only framework regions stay deferred).
parity_left=""
for cls in css cue html_template jsonnet pkl shell terraform text yaml; do
  if grep -q -F -e "\"$cls\": [\"ADR 0019\"" "$parity"; then
    parity_left="$parity_left $cls:still-deferred"
  fi
done
if [[ -z "$parity_left" ]] &&
  grep -q -F -e 'PARITY_DEFERRED = {' "$parity" &&
  grep -q -F -e '"vue": ["ADR 0019"' "$parity"; then
  ok
else
  bad "parity deferrals lost a file-family owner/route (css/template/shell/text/yaml/cue/jsonnet/pkl/terraform):$parity_left"
fi

# Delivered file-family tools: every cohort tool ID appears in
# REAL_ADAPTERS (delivered under #800; buf/qmlformat/qmllint delivered
# earlier under #799).
file_claim=""
for tool in shfmt shellcheck yamlfmt yamllint keep_sorted djlint stylelint jsonnetfmt; do
  if ! grep -q -F -e "\"$tool\":" "$adapters"; then
    file_claim="$file_claim $tool:missing"
  fi
done
if [[ -z "$file_claim" ]]; then
  ok
else
  bad "file-family adapter delivery missing:$file_claim (want all eight under #800)"
fi

# Every file-family class stays classified in the frozen taxonomy.
class_missing=""
for cls in css less scss html_template protobuf starlark toml yaml text shell cue jsonnet pkl qml terraform; do
  grep -q -F -e "\"$cls\":" "$adapters" || class_missing="$class_missing $cls:missing"
done
if [[ -z "$class_missing" ]]; then
  ok
else
  bad "frozen taxonomy lost a file-family class:$class_missing"
fi

# Starlark/Buildifier keeps adapter-backed fixture evidence.
if grep -q -F -e '"buildifier": {"format": ["starlark"]' "$adapters" &&
  grep -q -F -e 'matrix_starlark_lint_pass' "$matrix_data" &&
  grep -q -F -e 'matrix_starlark_format_fail' "$matrix_data" &&
  grep -q -F -e 'starlark' "$subjects" &&
  [[ -f "quality/adapter/src/parsers/buildifier.rs" ]]; then
  ok
else
  bad "starlark/buildifier lost its adapter plus matrix plus parser evidence"
fi

# TOML/Taplo keeps adapter-backed fixture evidence.
if grep -q -F -e '"taplo": {"format": ["toml"]' "$adapters" &&
  grep -q -F -e 'matrix_toml_lint_pass' "$matrix_data" &&
  grep -q -F -e 'matrix_toml_format_fail' "$matrix_data" &&
  grep -q -F -e 'toml' "$subjects" &&
  [[ -f "quality/adapter/src/parsers/taplo.rs" ]]; then
  ok
else
  bad "toml/taplo lost its adapter plus matrix plus parser evidence"
fi

# Curated defaults pin the two adapter-backed file families.
if grep -q -F -e '"starlark": {' "$curated" &&
  grep -q -F -e '"toml": {' "$curated" &&
  grep -q -F -e '"starlark": ["buildifier"]' "$curated" &&
  grep -q -F -e '"toml": ["taplo"]' "$curated"; then
  ok
else
  bad "curated defaults lost their starlark/toml pins"
fi

# Applicability never infers from suffix: provider classes intersect
# adapter support intersect policy, with registry validation only.
if grep -q -F -e 'effective classes =' "$sources_doc" &&
  grep -q -F -e 'target provider classes' "$sources_doc" &&
  grep -q -F -e 'intersect adapter-supported classes' "$sources_doc" &&
  grep -q -F -e 'Raw extension matching is registry validation, not an' "$integrations" &&
  grep -q -F -e 'never infer custom-rule sources' "$sources_doc"; then
  ok
else
  bad "applicability lost its never-inferred-from-suffix record"
fi

# Admissibility stays owner-contextual with the ambiguous cases pinned.
if grep -q -F -e '`BUILD` files admissible by basename' "$sources_doc" &&
  grep -q -F -e 'alone cannot decide `c` versus `cpp`' "$sources_doc" &&
  grep -q -F -e 'Extensionless requires shell rule/provider evidence' "$sources_doc" &&
  grep -q -F -e 'Requires template provider; bare `.html` without it is `html`' "$sources_doc" &&
  grep -q -F -e 'never inferred as fallback' "$sources_doc"; then
  ok
else
  bad "quality-sources lost its admissibility pins (BUILD/.h/extensionless/template/text)"
fi

# Known IDs plus single-family assignment cover every file-family class.
if grep -q -F -e '"css",' "$sources" &&
  grep -q -F -e '"protobuf",' "$sources" &&
  grep -q -F -e '"shell",' "$sources" &&
  grep -q -F -e '"yaml",' "$sources" &&
  grep -q -F -e '"text",' "$sources" &&
  grep -q -F -e '"starlark",' "$sources" &&
  grep -q -F -e '"toml",' "$sources" &&
  grep -q -F -e '"css": "css"' "$adapters" &&
  grep -q -F -e '"shell": "shell"' "$adapters"; then
  ok
else
  bad "known IDs or single-family assignment lost a file-family class"
fi

# Native-config bindings exist for the two backed file-family tools.
if grep -q -F -e 'buildifier_config' "$native" &&
  grep -q -F -e 'taplo_config' "$native"; then
  ok
else
  bad "native-config lost its buildifier/taplo bindings"
fi

# Real aspects keep the file-family stage wiring without a second config path.
if grep -q -F -e 'buildifier' "$aspects" &&
  grep -q -F -e 'taplo' "$aspects" &&
  grep -q -F -e 'target-coupled' "$aspects"; then
  ok
else
  bad "real_aspects lost its file-family stage wiring"
fi

# Owned gaps stay explicit: adapter execution plus pins plus platform/consumer/release.
# Functional: parity gate shape still fails closed on drift.
if grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" &&
  grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" &&
  grep -q -F -e 'no adapter class is unclassified' "$parity" &&
  grep -q -F -e 'no classified class lacks a disposition' "$parity"; then
  ok
else
  bad "parity gate lost its fail-closed shape"
fi

dx_test_summary "file-family quality qualification harness"
