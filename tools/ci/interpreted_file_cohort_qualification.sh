#!/usr/bin/env bash
# Interpreted/file-family-cohort qualification harness.
#
# Qualifies the as-built interpreted/file-family quality-cohort record with
# fixture evidence and owned gaps, without claiming Supported and without a
# false adapter claim:
# - delivered: decided release-assembled Ruby closure route for RuboCop and
#   StandardRB (the exceptional bundle within the approved packaging-effort
#   boundary; consumer path is download-verify-extract-execute only with the
#   bundle-vs-adapter qualification split recorded, never silent) plus decided
#   exact-module plus portable-`pwsh`-runtime route for PSScriptAnalyzer
#   (explicit-path import; console-parse versus library-API binding stays
#   open) plus frozen delivery-class routes for the file-family remainder
#   (cue/jsonnetfmt/pkl/modfmt/terraform/yamlfmt/keep-sorted as checksummed
#   standalone artifacts, djlint/yamllint as private wheel-only Python graph
#   members, Stylelint plus prettier-plugin-gherkin/sql/xml as private
# pure-JavaScript graph members; `protobuf`/`qml` stay owned
#   and are cross-linked here, never double-claimed; versions plus rule-sets
# qualified seed-only), initial artifact research rows as
# observations for digests (versions qualified seed-only),
#   adapter-input notes (RuboCop `--format json` versus text-parse,
#   `standardrb --fix`,
#   PSScriptAnalyzer console versus library API, whole-file rewrite versus
#   check-only fix modes with the provisional sandbox-apply-and-diff flow,
#   Buildifier/Taplo/Vale probes staying provisional), native-config defaults
# qualified seed-only (whole-file rewrite versus check-only
#   per tool with no auto-supplied preset, suffix inference rejected),
#   parity-deferred
#   ruby/powershell/cue/jsonnet/pkl/css/html_template/gherkin/sql/xml/
#   go_module/terraform/yaml/text with owner plus frozen route,
#   classification-only taxonomy with no curated defaults and no
#   native-config binding;
# - open under with honest records: exact bundle contents/lock inputs
#   plus module/runtime digests plus parser plus runner-matrix pass/fail
#   plus fix/format evidence per adapter-backed class, native-config
#   qualification against the native-config contract, platform plus
#   consumer plus release evidence. REAL_ADAPTERS claims a cohort class only
#   when green.
#
# Versioned here, run by CI via `bazel run //tools/ci:interpreted_file_cohort_qualification`,
# following //tools/ci:structured_cohort_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
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

# No false adapter claim for the cohort: none of the cohort tool IDs appear
# as adapter entries in REAL_ADAPTERS. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not. (The `"tool": {` shape
# matches adapter entries only: class==tool taxonomy rows like
# `"cue": "cue"` carry a family string, never a capability map, so they
# cannot match here. `protobuf`/`qml` tools stay owned and
# are never claimed here either.)
cohort_claim=""
for tool in rubocop standardrb standard psscriptanalyzer cue jsonnetfmt pkl djlint stylelint modfmt terraform yamlfmt yamllint keep-sorted keep_sorted buf qmlformat qmllint; do
  if grep -q -F -e "\"$tool\": {" "$adapters"; then
    cohort_claim="$cohort_claim $tool:claimed"
  fi
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "false adapter claim for interpreted/file-family cohort:$cohort_claim"
fi

# Parity deferrals own the cohort classes with owner plus frozen route plus
# the live-successor record (closed owns nothing here; `protobuf`/
# `qml` stay owned and are not re-owned here).
cohort_deferred=""
for cls in ruby powershell cue jsonnet pkl css html_template gherkin sql xml go_module terraform yaml text; do
  grep -q -F -e "\"$cls\":" "$parity" || cohort_deferred="$cohort_deferred $cls:missing"
done
if [[ -z "$cohort_deferred" ]] &&
  grep -q -F -e '"ruby": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"powershell": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'foundation deferred by ADR 0019' "$parity" &&
  grep -q -F -e 'Interpreted/file-family cohort owned by issue #420' "$parity"; then
  ok
else
  bad "parity deferrals lost the interpreted/file-family owner plus frozen route plus #420 record:$cohort_deferred"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
class_missing=""
for cls in ruby powershell cue jsonnet pkl css less scss html_template gherkin sql xml go_module terraform yaml text; do
  grep -q -F -e "\"$cls\":" "$adapters" || class_missing="$class_missing $cls:missing"
done
if [[ -z "$class_missing" ]]; then
  ok
else
  bad "frozen taxonomy lost an interpreted/file-family class:$class_missing"
fi

# Classification-only today: cohort families carry no curated defaults
# (curated membership unchanged; no native default selected).
cohort_curated=""
for family in '"ruby": {' '"powershell": {' '"cue": {' '"jsonnet": {' '"pkl": {' '"css": {' '"html_template": {' '"gherkin": {' '"sql": {' '"xml": {' '"go_module": {' '"terraform": {' '"text": {' '"yaml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim an interpreted/file-family family before adapters land:$cohort_curated"
fi

# No hidden cohort native-config preset: no cohort binding exists in the
# typed native-config rules (adapters run pinned upstream defaults until
# qualifies checked-in policy against the native-config contract; the
# provisional RuboCop/StandardRB plus PSScriptAnalyzer plus file-family
# suggestions stay review inputs, never supplied configs).
cohort_config=""
for tool in rubocop standardrb standard psscriptanalyzer cue jsonnetfmt pkl djlint stylelint modfmt terraform yamlfmt yamllint keep-sorted ruby powershell; do
  if grep -q -F -e "${tool}_config" "$native"; then
    cohort_config="$cohort_config $tool:preset"
  fi
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config carries a hidden interpreted/file-family preset:$cohort_config"
fi

# No false green claim: the runner matrix carries no cohort cells yet, so
# REAL_ADAPTERS cannot claim these classes (claims land only with green
# pass/fail plus fix/format evidence per adapter-backed class).
cohort_matrix=""
for cell in matrix_ruby_ matrix_powershell_ matrix_cue_ matrix_jsonnet_ matrix_pkl_ matrix_css_ matrix_html_template_ matrix_gherkin_ matrix_sql_ matrix_xml_ matrix_go_module_ matrix_terraform_ matrix_yaml_ matrix_text_; do
  if grep -q -F -e "$cell" "$matrix"; then
    cohort_matrix="$cohort_matrix $cell:claimed"
  fi
done
if [[ -z "$cohort_matrix" ]]; then
  ok
else
  bad "runner matrix claims an interpreted/file-family cell without adapter qualification:$cohort_matrix"
fi

# Tool acquisition keeps the decided release-assembled Ruby closure route
# (exceptional bundle within the approved packaging-effort boundary) with
# bundle contents plus lock inputs pending, the bundle-vs-adapter split
# recorded, and no false claim, owned by (live successor to closed
# for the ruby class).
if grep -q -F -e 'Decided route: RuboCop and StandardRB take the' "$acquisition" &&
  grep -q -F -e 'release-assembled Ruby closure route (the exceptional bundle' "$acquisition" &&
  grep -q -F -e 'Bundle contents, lock inputs, and' "$acquisition" &&
  grep -q -F -e 'closed #307 for the `ruby`' "$acquisition" &&
  grep -q -F -e 'no adapter claims `ruby` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #420)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Ruby closure route, bundle honesty, or #420 ownership or no-claim honesty"
fi

# Tool acquisition keeps the decided exact-module plus portable-pwsh route
# for PSScriptAnalyzer with the console-parse versus library-API decision
# open and no false claim, owned by.
if grep -q -F -e 'Decided route: PSScriptAnalyzer takes the' "$acquisition" &&
  grep -q -F -e 'exact-module plus portable-PowerShell-runtime route' "$acquisition" &&
  grep -q -F -e 'console-parse versus library-API binding choice' "$acquisition" &&
  grep -q -F -e 'live successor to closed #307 for the `powershell`' "$acquisition" &&
  grep -q -F -e 'no adapter claims `powershell` yet' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided PSScriptAnalyzer route, binding-decision honesty, or #420 ownership or no-claim honesty"
fi

# Tool acquisition keeps initial artifact research rows for the cohort with
# byte-identity risk explicit; file-family versions qualified seed-only under
# with digests as observations, Ruby/PowerShell bundle/module
# identities stay pending under.
cohort_research=""
for tool in '| RuboCop |' '| StandardRB |' '| PSScriptAnalyzer |' '| pwsh |' '| cue |' '| jsonnetfmt |' '| pkl |' '| djlint |' '| Stylelint |' '| prettier-plugin-gherkin |' '| prettier-plugin-sql |' '| prettier-plugin-xml |' '| modfmt |' '| terraform |' '| yamlfmt |' '| yamllint |' '| keep-sorted |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'owned by issue #420' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #420' "$acquisition" &&
  grep -q -F -e 'must establish and record' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost an interpreted/file-family research row or its #489 versions plus #420 digests honesty:$cohort_research"
fi

# Tool integrations keep the cohort adapter-input notes with file-family
# versions qualified under and digests as observations:
# release-assembled closure with download-verify-extract-execute plus the
# bundle-vs-adapter split, PSScriptAnalyzer console versus library API,
# frozen file-family delivery classes with `protobuf`/`qml` cross-linked to
# (never double-claimed), whole-file rewrite versus check-only fix
# modes, Buildifier/Taplo/Vale probes staying provisional, file-family
# versions qualified with no adapter claim.
if grep -q -F -e '**Interpreted/file-family cohort (issue #420' "$integrations" &&
  grep -q -F -e 'no adapter claims `ruby`,' "$integrations" &&
  grep -q -F -e 'download-verify-extract-execute' "$integrations" &&
  grep -q -F -e 'bundle-vs-adapter' "$integrations" &&
  grep -q -F -e 'console-parse versus library-API' "$integrations" &&
  grep -q -F -e 'cross-linked here, never double-claimed' "$integrations" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$integrations" &&
  grep -q -F -e 'Buildifier/Taplo/Vale probes stay provisional' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #420' "$integrations"; then
  ok
else
  bad "tool-integrations lost its interpreted/file-family notes with #489 versions plus #420 adapters split"
fi

# Support matrix keeps the cohort routes plus qualified file-family defaults
# plus adapter-input notes plus cohort tracking, all citing 
# for adapters/digests without approving hidden presets or claiming support.
if grep -q -F -e 'release-assembled Ruby closure route (issue #420' "$support" &&
  grep -q -F -e 'portable-PowerShell-runtime route (issue #420' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$support" &&
  grep -q -F -e 'quality/tests/fixtures/file_family_quality/pins.bzl' "$support" &&
  grep -q -F -e 'tracked' "$support" &&
  grep -q -F -e 'under issue #420' "$support" &&
  grep -q -F -e 'owned by issue #420' "$support" &&
  grep -q -F -e 'itemized under issue #420' "$support" &&
  grep -q -F -e 'stay open' "$support" &&
  grep -q -F -e '(issue #420)' "$support" &&
  grep -q -F -e 'to issue' "$support" &&
  grep -q -F -e '#420' "$support"; then
  ok
else
  bad "support-matrix lost its interpreted/file-family routes, qualified defaults, adapter notes, or #420 cohort tracking"
fi

# Tool baseline keeps the Ruby/PowerShell/file-family coverage rows
# (integration inventory, not a support claim).
baseline_missing=""
for row in '| Ruby | | RuboCop, StandardRB |' '| PowerShell | | PSScriptAnalyzer |' '| CUE | cue fmt |' '| Jsonnet | jsonnetfmt |' '| Pkl | pkl |' '| HTML templates | djlint |' '| Terraform | terraform fmt |' '| YAML | yamlfmt | yamllint |' '| CSS, Less, SCSS | Prettier | Stylelint |' '| Gherkin | prettier-plugin-gherkin |' '| SQL | prettier-plugin-sql |' '| XML | prettier-plugin-xml |' '| Go modules | modfmt |'; do
  grep -q -F -e "$row" "$baseline" || baseline_missing="$baseline_missing $row:missing"
done
if [[ -z "$baseline_missing" ]] && grep -q -F -e 'keep-sorted' "$baseline"; then
  ok
else
  bad "tool-baseline lost a Ruby/PowerShell/file-family coverage row:$baseline_missing"
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

dx_test_summary "Interpreted/file-family-cohort qualification harness"
