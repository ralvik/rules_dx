#!/usr/bin/env bash
# File-family quality defaults qualification harness.
#
# Qualifies the provisional file-family format plus lint defaults against the
# native-configuration contract with no hidden presets. covers
# adapters (plus digests), not versions: this harness owns versions plus
# rule-sets.
# - pinned: cue v0.17.1, jsonnetfmt v0.22.0 (go-jsonnet; C++ v0.21.0 observed
#   not pinned), pkl 0.32.1, terraform v1.16.1, djlint v1.45.0, stylelint
#   17.14.1, prettier 3.9.6 with prettier-plugin-sql 0.15.1 plus
#   prettier-plugin-gherkin 4.0.0 plus @prettier/plugin-xml 3.4.2 plus modfmt
#   v0.4.0 from github.com/joshdk/modfmt in
#   `quality/tests/fixtures/file_family_quality/pins.bzl` (modfmt upstream
# identity resolved seed-only under, gherkin/xml rechecked latest
# stable at implementation under; living at head rejected);
#   frozen delivery-class identities recorded (standalone checksummed
#   artifacts for cue/jsonnetfmt/pkl/terraform/yamlfmt/keep-sorted/modfmt,
#   private wheel-only Python graph for djlint/yamllint, private
#   pure-JavaScript graph for stylelint/plugins), digests stay owned under
# ; `protobuf`/`qml` stay owned, cross-linked
#   here never double-claimed.
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. cue fmt plus jsonnetfmt plus pkl plus
#   terraform fmt plus modfmt are whole-file rewrite with no rule-set
#   selection; djlint --lint versus --reformat are upstream modes with no
#   auto-supplied preset; stylelint plus yamllint default rulesets are
#   upstream built-in defaults with no auto-supplied config preset;
#   yamlfmt -lint is the upstream built-in check mode with native config
#   discovery; keep-sorted is check-only; prettier-plugin closures ride
#   Prettier 3.9.6 whole-file rewrite. Suffix inference rejected: registry
#   owns applicability. Beyond-default auto presets plus --enable=all-style
#   maxima rejected.
# - fixtures: `quality/tests/fixtures/file_family_quality/` pins plus BUILD;
#   no file-family native-config preset, no adapter claim, no curated
#   defaults, no matrix cells; deferred foundations have no hello bazel test,
#   so the fixture pair plus `bazel build` of the fixture is the live proof
#   (no hello green here).
# - open owned gaps: digests plus adapters under, platform plus consumer
#   plus release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:file_family_defaults_qualification`,
# following //tools/ci:structured_defaults_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="quality/tests/fixtures/file_family_quality/pins.bzl"
pins_build="quality/tests/fixtures/file_family_quality/BUILD.bazel"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture pair plus pins stay present.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "file-family quality fixture missing (want $pins plus $pins_build)"
fi

# Pins record the qualified upstream versions
# gherkin/xml resolved seed-only under).
if grep -q -F -e 'CUE_VERSION = "v0.17.1"' "$pins" &&
  grep -q -F -e 'JSONNETFMT_VERSION = "v0.22.0"' "$pins" &&
  grep -q -F -e 'PKL_VERSION = "0.32.1"' "$pins" &&
  grep -q -F -e 'TERRAFORM_VERSION = "v1.16.1"' "$pins" &&
  grep -q -F -e 'DJLINT_VERSION = "v1.45.0"' "$pins" &&
  grep -q -F -e 'STYLELINT_VERSION = "17.14.1"' "$pins" &&
  grep -q -F -e 'PRETTIER_VERSION = "3.9.6"' "$pins" &&
  grep -q -F -e 'PRETTIER_PLUGIN_GHERKIN_VERSION = "4.0.0"' "$pins" &&
  grep -q -F -e 'PRETTIER_PLUGIN_SQL_VERSION = "0.15.1"' "$pins" &&
  grep -q -F -e 'PRETTIER_PLUGIN_XML_VERSION = "3.4.2"' "$pins" &&
  grep -q -F -e 'MODFMT_VERSION = "v0.4.0"' "$pins" &&
  grep -q -F -e 'YAMLFMT_VERSION = "v0.21.0"' "$pins" &&
  grep -q -F -e 'YAMLLINT_VERSION = "1.38.0"' "$pins" &&
  grep -q -F -e 'KEEP_SORTED_VERSION = "v0.10.0"' "$pins"; then
  ok
else
  bad "pins.bzl lost its qualified file-family versions under issues #489 plus #582"
fi

# Pins record the frozen delivery-class identities plus resolutions.
if grep -q -F -e 'standalone checksummed release artifact' "$pins" &&
  grep -q -F -e 'private wheel-only Python graph member' "$pins" &&
  grep -q -F -e 'private pure-JavaScript graph member' "$pins" &&
  grep -q -F -e 'C++ jsonnet v0.21.0 observed, not pinned' "$pins" &&
  grep -q -F -e 'modfmt upstream identity resolved seed-only under issue #582' "$pins" &&
  grep -q -F -e 'github.com/joshdk/modfmt v0.4.0' "$pins" &&
  grep -q -F -e 'prettier-plugin-gherkin 4.0.0 plus @prettier/plugin-xml 3.4.2 pinned seed-only under issue #582' "$pins" &&
  grep -q -F -e 'living at head rejected' "$pins" &&
  grep -q -F -e 'protobuf plus qml stay owned by issue #488' "$pins"; then
  ok
else
  bad "pins.bzl lost its delivery-class identities plus #582 resolutions"
fi

# Pins record the sole-policy plus qualified rule-set resolutions plus rejections.
if grep -q -F -e 'NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"' "$pins" &&
  grep -q -F -e 'whole-file rewrite with check/diff mode' "$pins" &&
  grep -q -F -e 'no auto-supplied preset' "$pins" &&
  grep -q -F -e 'default ruleset is the upstream built-in default ruleset' "$pins" &&
  grep -q -F -e 'suffix inference rejected' "$pins" &&
  grep -q -F -e 'beyond-default switches rejected' "$pins" &&
  grep -q -F -e 'hidden presets rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its sole-policy plus rule-set resolutions plus rejections under issue #489"
fi

# No hidden file-family native-config preset: no cohort binding exists in
# the typed native-config rules (adapters run pinned upstream defaults until
# checked-in policy qualifies against the native-config contract).
family_config=""
for tool in cue jsonnetfmt pkl djlint stylelint modfmt terraform yamlfmt yamllint keep-sorted keep_sorted prettier gherkin sql xml css html_template go_module text yaml; do
  if grep -q -F -e "${tool}_config" "$native"; then
    family_config="$family_config $tool:preset"
  fi
done
if [[ -z "$family_config" ]]; then
  ok
else
  bad "native-config carries a hidden file-family preset:$family_config"
fi

# No false adapter claim for the remaining file-family cohort: none of the cohort
# tool IDs appear in REAL_ADAPTERS. Classification exists; adapter claim
# does not (buf/qmlformat/qmllint delivered under #799, no longer guarded here).
# The `"tool": {` shape matches adapter entries only: class==tool taxonomy
# rows like `"cue": "cue"` carry a family string, never a capability map,
# so they cannot match here.
family_claim=""
for tool in cue jsonnetfmt pkl djlint stylelint modfmt terraform yamlfmt yamllint keep-sorted keep_sorted; do
  if grep -q -F -e "\"$tool\": {" "$adapters"; then
    family_claim="$family_claim $tool:claimed"
  fi
done
if [[ -z "$family_claim" ]]; then
  ok
else
  bad "false adapter claim for file-family cohort:$family_claim"
fi

# Classification-only today: file-family families carry no curated defaults,
# no runner-matrix cells (claims land only with green adapter evidence
#
family_curated=""
for family in '"cue": {' '"jsonnet": {' '"pkl": {' '"css": {' '"html_template": {' '"gherkin": {' '"sql": {' '"xml": {' '"go_module": {' '"terraform": {' '"text": {' '"yaml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    family_curated="$family_curated $family:claimed"
  fi
done
if [[ -z "$family_curated" ]] &&
  ! grep -q -F -e 'matrix_cue_' "$matrix" &&
  ! grep -q -F -e 'matrix_terraform_' "$matrix" &&
  ! grep -q -F -e 'matrix_yaml_' "$matrix"; then
  ok
else
  bad "curated or matrix claims a file-family family before adapters land:$family_curated"
fi

# Support matrix keeps the qualified file-family versions plus rule-sets
# with fixtures and harness; digests plus adapters stay under.
# Support matrix resolves the rule-selection plus config-discovery plus
# suffix-inference conflicts as upstream built-in defaults with registry
# applicability (no auto-supplied preset, never suffix-inferred).
# Tool acquisition keeps the frozen standalone route with no separate
# ambient resolution and no false claim; versions qualified under with
# modfmt resolved under, digests plus adapters stay pending under.
if grep -q -F -e 'Decided route: cue, jsonnetfmt, pkl, terraform, yamlfmt, keep-sorted, and modfmt take the' "$acquisition" &&
  grep -q -F -e 'standalone checksummed' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$acquisition" &&
  grep -q -F -e 'modfmt v0.4.0' "$acquisition" &&
  grep -q -F -e 'resolved seed-only under issue #582' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #420' "$acquisition" &&
  grep -q -F -e 'no adapter claims `cue`' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its frozen standalone route with #489 plus #582 versions plus #420 digests/adapters split"
fi

# Tool acquisition keeps the frozen Python plus Node graph routes with the
# private-closure discipline and no false claim; versions qualified under 
# with gherkin/xml resolved under.
if grep -q -F -e 'Decided route: djlint and yamllint take the' "$acquisition" &&
  grep -q -F -e 'private wheel-only Python graph' "$acquisition" &&
  grep -q -F -e 'Decided route: Stylelint plus prettier-plugin' "$acquisition" &&
  grep -q -F -e 'private pure-JavaScript graph' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$acquisition" &&
  grep -q -F -e 'prettier-plugin-gherkin 4.0.0' "$acquisition" &&
  grep -q -F -e '@prettier/plugin-xml 3.4.2' "$acquisition" &&
  grep -q -F -e 'resolved seed-only under issue #582' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #420' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its frozen Python/Node routes with #489 plus #582 versions plus #420 digests/adapters split"
fi

# Tool acquisition keeps the file-family research rows with byte-identity risk
# (modfmt plus gherkin/xml resolved seed-only under).
family_research=""
for tool in '| cue |' '| jsonnetfmt |' '| pkl |' '| djlint |' '| Stylelint |' '| prettier-plugin-gherkin |' '| prettier-plugin-sql |' '| prettier-plugin-xml |' '| modfmt |' '| terraform |' '| yamlfmt |' '| yamllint |' '| keep-sorted |'; do
  grep -q -F -e "$tool" "$acquisition" || family_research="$family_research $tool:missing"
done
if [[ -z "$family_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a file-family research row or byte-identity honesty:$family_research"
fi

# Tool integrations keep the file-family adapter-input notes with pinned
# versions plus resolutions (adapters still open under; protobuf/qml cross-linked to).
if grep -q -F -e 'Interpreted/file-family cohort (issue #420' "$integrations" &&
  grep -q -F -e 'no adapter claims `ruby`,' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #489' "$integrations" &&
  grep -q -F -e 'resolved seed-only under issue #582' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #420' "$integrations" &&
  grep -q -F -e 'cross-linked here, never double-claimed' "$integrations"; then
  ok
else
  bad "tool-integrations lost its file-family notes with #489 plus #582 versions plus #420 adapters split"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "file_family_defaults_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:file_family_defaults_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the file_family_defaults_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the file-family fixture loads on the seed host (deferred
# families have no hello bazel test; foundations feasibility or N/A).
if bazel build //quality/tests/fixtures/file_family_quality:corpus_starlark --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "file-family quality live proof failed (want fixture corpus_starlark build green)"
fi

dx_test_summary "file-family defaults qualification harness"
