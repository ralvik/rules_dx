#!/usr/bin/env bash
# File-family quality qualification harness (issue #313).
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
# - open under #313 with honest records: adapter execution for the
#   deferred families, platform plus consumer plus release evidence,
#   exact pins/digests/rule-sets/adapter mappings per family.
#
# Versioned here, run by CI via `bazel run //tools/ci:file_family_qualification`,
# following //tools/ci:consumer_ci_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

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
subjects="quality/testdata/BUILD.bazel"
aspects="quality/real_aspects.bzl"
verify="docs/testing/verification-matrix.md"

# Support matrix owns the Quality And File Families section with the
# qualified seed-only record under #313.
if grep -q -F -e '## Quality And File Families' "$support" \
  && grep -q -F -e 'qualified seed-only under issue #313' "$support" \
  && grep -q -F -e 'bazel run //tools/ci:file_family_qualification' "$support"; then
  ok
else
  bad "support-matrix lost its Quality And File Families qualified record under #313"
fi

# Support matrix keeps every file-family row from the issue scope.
if grep -q -F -e '| CSS, Less, SCSS |' "$support" \
  && grep -q -F -e '| HTML templates |' "$support" \
  && grep -q -F -e '| Protocol Buffer |' "$support" \
  && grep -q -F -e '| Starlark |' "$support" \
  && grep -q -F -e '| TOML |' "$support" \
  && grep -q -F -e '| YAML |' "$support" \
  && grep -q -F -e '| Language-independent text |' "$support" \
  && grep -q -F -e '| Shell |' "$support"; then
  ok
else
  bad "support-matrix lost a file-family row (CSS/templates/protobuf/starlark/toml/yaml/text/shell)"
fi

# Support matrix keeps the feasibility rows for cue/jsonnet/pkl/qml/terraform.
if grep -q -F -e '| CUE |' "$support" \
  && grep -q -F -e '| Jsonnet |' "$support" \
  && grep -q -F -e '| Pkl |' "$support" \
  && grep -q -F -e '| QML |' "$support" \
  && grep -q -F -e '| Terraform |' "$support" \
  && grep -q -F -e 'Planned: feasibility' "$support"; then
  ok
else
  bad "support-matrix lost a feasibility row (cue/jsonnet/pkl/qml/terraform)"
fi

# Support matrix names the intended per-family tools without claiming Supported.
if grep -q -F -e 'Planned: Prettier and Stylelint' "$support" \
  && grep -q -F -e 'Planned: djlint' "$support" \
  && grep -q -F -e 'Planned: buf format and lint' "$support" \
  && grep -q -F -e 'Planned: Buildifier' "$support" \
  && grep -q -F -e 'Planned: Taplo format and lint' "$support" \
  && grep -q -F -e 'Planned: yamlfmt and yamllint' "$support" \
  && grep -q -F -e 'Planned: keep-sorted' "$support" \
  && grep -q -F -e 'Planned: shfmt and ShellCheck' "$support"; then
  ok
else
  bad "support-matrix lost a per-family tool cell (prettier/stylelint/djlint/buf/buildifier/taplo/yaml/keep-sorted/shfmt)"
fi

# Support matrix keeps no Supported claim and owns N/A applicability honesty.
if ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$support" | grep -q . \
  && grep -q -F -e 'no cell is currently' "$support" \
  && grep -q -F -e 'Applicability verification' "$support" \
  && grep -q -F -e 'rather than inferred from a file suffix' "$support"; then
  ok
else
  bad "support-matrix lost its no-Supported plus applicability-verification record"
fi

# Tool baseline owns the matching file-family rows.
if grep -q -F -e '| CSS, Less, SCSS | Prettier | Stylelint |' "$baseline" \
  && grep -q -F -e '| HTML templates | djlint |' "$baseline" \
  && grep -q -F -e '| Protocol Buffer | buf format | buf lint |' "$baseline" \
  && grep -q -F -e '| Starlark | Buildifier | Buildifier |' "$baseline" \
  && grep -q -F -e '| TOML | Taplo | Taplo |' "$baseline" \
  && grep -q -F -e '| YAML | yamlfmt | yamllint |' "$baseline" \
  && grep -q -F -e 'keep-sorted' "$baseline" \
  && grep -q -F -e '| Shell | shfmt | ShellCheck |' "$baseline"; then
  ok
else
  bad "tool-baseline lost a file-family row (css/djlint/buf/buildifier/taplo/yaml/keep-sorted/shell)"
fi

# Tool baseline keeps the feasibility tool names for cue/jsonnet/pkl/qml/terraform.
if grep -q -F -e '| CUE | cue fmt |' "$baseline" \
  && grep -q -F -e '| Jsonnet | jsonnetfmt |' "$baseline" \
  && grep -q -F -e '| Pkl | pkl |' "$baseline" \
  && grep -q -F -e '| QML | qmlformat | qmllint |' "$baseline" \
  && grep -q -F -e '| Terraform | terraform fmt |' "$baseline"; then
  ok
else
  bad "tool-baseline lost a feasibility row (cue/jsonnet/pkl/qml/terraform)"
fi

# Tool acquisition routes every file-family tool to a frozen delivery class.
if grep -q -F -e 'Private pure-JavaScript graph plus shared managed Node' "$acquisition" \
  && grep -q -F -e 'Stylelint' "$acquisition" \
  && grep -q -F -e 'Private wheel-only Python graph plus shared managed Python' "$acquisition" \
  && grep -q -F -e 'djlint' "$acquisition" \
  && grep -q -F -e 'Checksummed native/self-contained artifact' "$acquisition" \
  && grep -q -F -e 'keep-sorted' "$acquisition" \
  && grep -q -F -e 'shfmt' "$acquisition" \
  && grep -q -F -e 'yamlfmt' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a file-family delivery route (node/python/standalone)"
fi

# Tool acquisition keeps the decided buf route with no false adapter claim.
if grep -q -F -e 'Decided route: `buf` takes the checksummed' "$acquisition" \
  && grep -q -F -e 'no adapter claims `protobuf` yet' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided buf route or no-claim honesty"
fi

# Tool acquisition keeps the qml authoritative-toolchain route with no false claim.
if grep -q -F -e 'qmlformat and qmllint take the authoritative-toolchain route' "$acquisition" \
  && grep -q -F -e 'no adapter claiming `qml` yet' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its qml route or no-claim honesty"
fi

# Parity deferrals name owner plus frozen route for every file-family class.
if grep -q -F -e '"css": ["O32"' "$parity" \
  && grep -q -F -e '"html_template": ["O32"' "$parity" \
  && grep -q -F -e '"protobuf": ["O32"' "$parity" \
  && grep -q -F -e '"shell": ["O32"' "$parity" \
  && grep -q -F -e '"text": ["O32"' "$parity" \
  && grep -q -F -e '"yaml": ["O32"' "$parity" \
  && grep -q -F -e '"cue": ["O32"' "$parity" \
  && grep -q -F -e '"jsonnet": ["O32"' "$parity" \
  && grep -q -F -e '"pkl": ["O32"' "$parity" \
  && grep -q -F -e '"qml": ["O32"' "$parity" \
  && grep -q -F -e '"terraform": ["O32"' "$parity" \
  && grep -q -F -e 'PARITY_DEFERRED = {' "$parity"; then
  ok
else
  bad "parity deferrals lost a file-family owner/route (css/template/protobuf/shell/text/yaml/cue/jsonnet/pkl/qml/terraform)"
fi

# No false adapter claim for the deferred file-family tools (tool IDs only;
# class==tool names like cue/pkl/terraform live in the taxonomy, not here).
file_claim=""
for tool in buf qmlformat qmllint shfmt shellcheck yamlfmt yamllint keep-sorted keep_sorted djlint stylelint jsonnetfmt; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    file_claim="$file_claim $tool:claimed"
  fi
done
if [[ -z "$file_claim" ]]; then
  ok
else
  bad "false adapter claim for deferred file-family tools:$file_claim"
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
if grep -q -F -e '"buildifier": {"format": ["starlark"]' "$adapters" \
  && grep -q -F -e 'matrix_starlark_lint_pass' "$matrix" \
  && grep -q -F -e 'matrix_starlark_format_fail' "$matrix" \
  && grep -q -F -e 'starlark' "$subjects" \
  && [[ -f "quality/adapter/src/parsers/buildifier.rs" ]]; then
  ok
else
  bad "starlark/buildifier lost its adapter plus matrix plus parser evidence"
fi

# TOML/Taplo keeps adapter-backed fixture evidence.
if grep -q -F -e '"taplo": {"format": ["toml"]' "$adapters" \
  && grep -q -F -e 'matrix_toml_lint_pass' "$matrix" \
  && grep -q -F -e 'matrix_toml_format_fail' "$matrix" \
  && grep -q -F -e 'toml' "$subjects" \
  && [[ -f "quality/adapter/src/parsers/taplo.rs" ]]; then
  ok
else
  bad "toml/taplo lost its adapter plus matrix plus parser evidence"
fi

# Curated defaults pin the two adapter-backed file families.
if grep -q -F -e '"starlark": {' "$curated" \
  && grep -q -F -e '"toml": {' "$curated" \
  && grep -q -F -e '"starlark": ["buildifier"]' "$curated" \
  && grep -q -F -e '"toml": ["taplo"]' "$curated"; then
  ok
else
  bad "curated defaults lost their starlark/toml pins"
fi

# Applicability never infers from suffix: provider classes intersect
# adapter support intersect policy, with registry validation only.
if grep -q -F -e 'effective classes =' "$applicability" \
  && grep -q -F -e 'target provider classes' "$applicability" \
  && grep -q -F -e 'intersect adapter-supported classes' "$applicability" \
  && grep -q -F -e 'Raw extension matching is registry validation, not an' "$integrations" \
  && grep -q -F -e 'never infer custom-rule sources' "$sources_doc"; then
  ok
else
  bad "applicability lost its never-inferred-from-suffix record"
fi

# Admissibility stays owner-contextual with the ambiguous cases pinned.
if grep -q -F -e '`BUILD` files admissible by basename' "$sources_doc" \
  && grep -q -F -e 'alone cannot decide `c` versus `cpp`' "$sources_doc" \
  && grep -q -F -e 'Extensionless requires shell rule/provider evidence' "$sources_doc" \
  && grep -q -F -e 'Requires template provider; bare `.html` without it is `html`' "$sources_doc" \
  && grep -q -F -e 'never inferred as fallback' "$sources_doc"; then
  ok
else
  bad "quality-sources lost its admissibility pins (BUILD/.h/extensionless/template/text)"
fi

# Known IDs plus single-family assignment cover every file-family class.
if grep -q -F -e '"css",' "$sources" \
  && grep -q -F -e '"protobuf",' "$sources" \
  && grep -q -F -e '"shell",' "$sources" \
  && grep -q -F -e '"yaml",' "$sources" \
  && grep -q -F -e '"text",' "$sources" \
  && grep -q -F -e '"starlark",' "$sources" \
  && grep -q -F -e '"toml",' "$sources" \
  && grep -q -F -e '"css": "css"' "$adapters" \
  && grep -q -F -e '"shell": "shell"' "$adapters"; then
  ok
else
  bad "known IDs or single-family assignment lost a file-family class"
fi

# Native-config bindings exist for the two backed file-family tools.
if grep -q -F -e 'buildifier_config' "$native" \
  && grep -q -F -e 'taplo_config' "$native"; then
  ok
else
  bad "native-config lost its buildifier/taplo bindings"
fi

# Real aspects keep the file-family stage wiring without a second config path.
if grep -q -F -e 'buildifier' "$aspects" \
  && grep -q -F -e 'taplo' "$aspects" \
  && grep -q -F -e 'target-coupled' "$aspects"; then
  ok
else
  bad "real_aspects lost its file-family stage wiring"
fi

# Verification matrix keeps the qualified record with no Supported claim.
if grep -q -F -e 'file_family_qualification' "$verify" \
  && grep -q -F -e 'qualified seed-only under #313' "$verify" \
  && ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$verify" | grep -q .; then
  ok
else
  bad "verification-matrix lost its #313 qualified record or gained Supported"
fi

# Owned gaps stay explicit: adapter execution plus pins plus platform/consumer/release.
if grep -q -F -e 'adapter execution' "$support" \
  && grep -q -F -e 'exact pins' "$support" \
  && grep -q -F -e 'platform plus consumer plus release' "$support"; then
  ok
else
  bad "support-matrix lost its file-family owned-gap honesty"
fi

# Functional: parity gate shape still fails closed on drift.
if grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" \
  && grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" \
  && grep -q -F -e 'no adapter class is unclassified' "$parity" \
  && grep -q -F -e 'no classified class lacks a disposition' "$parity"; then
  ok
else
  bad "parity gate lost its fail-closed shape"
fi

echo "file-family quality qualification harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
