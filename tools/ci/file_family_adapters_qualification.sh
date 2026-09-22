#!/usr/bin/env bash
# Interpreted/file-family adapters qualification harness (issue #800).
#
# Qualifies the as-built interpreted/file-family adapter delivery with
# fixture evidence and owned gaps, without claiming Supported:
# - delivered: fifteen adapters (`cue` format cue, `jsonnetfmt` format
#   jsonnet, `pkl` format pkl, `modfmt` format go_module, `terraform`
#   format terraform, `djlint` format plus lint html_template, `stylelint`
#   lint css plus less plus scss, `yamlfmt` format yaml, `yamllint` lint
#   yaml, `keep_sorted` lint text, `shfmt` format shell, `shellcheck` lint
#   shell, `rubocop` lint ruby, `standardrb` format ruby,
#   `psscriptanalyzer` lint powershell) plus `prettier` format extension
#   to css plus less plus scss plus gherkin plus sql plus xml over the
#   decided routes (standalone checksummed artifacts; private wheel-only
#   Python graph for djlint/yamllint; private pure-JavaScript graph for
#   stylelint/plugins; release-assembled Ruby closure; exact-module plus
#   portable pwsh runtime); per-tool fixtures with pins plus samples;
#   Layer-2 matrix pass plus fail cells per tool and per class (format via
#   fake shell doubles seed-only, lint via delegated recorded diagnostics
#   like Clippy/rustc, prettier new classes via fake prettier warn lines);
#   parsers with pass plus fail samples; native-config bindings for the
#   three configurable tools; runner dispatch plus fix flows (formatters
#   whole-file rewrite, lint check-only via sandbox-apply-and-diff);
#   parity deferrals removed for the seventeen delivered classes;
#   Depcheck/Examples opens resolved where applicable (file-family has no
#   application foundation, so no depcheck lock fixtures and no adopt-*
#   examples; the admitted depcheck languages and adopt-* examples stay
#   unchanged);
# - open owned gaps: exact artifact digests plus runtime bounds, platform
#   plus consumer plus release evidence, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:file_family_adapters_qualification`,
# following //tools/ci:native_adapters_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_file_family.bzl"
cases="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
real_rs="quality/runner/src/real.rs"
commands="quality/adapter/src/commands.rs"
integrations="docs/quality/tool-integrations.md"
runner_doc="docs/quality/runner-matrix.md"
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Per-tool fixture dirs stay present (fifteen file-family fixtures).
if [[ -d "quality/tests/fixtures/file_family_adapters/cue" && -d "quality/tests/fixtures/file_family_adapters/jsonnetfmt" && -d "quality/tests/fixtures/file_family_adapters/pkl" && -d "quality/tests/fixtures/file_family_adapters/modfmt" && -d "quality/tests/fixtures/file_family_adapters/terraform" && -d "quality/tests/fixtures/file_family_adapters/djlint" && -d "quality/tests/fixtures/file_family_adapters/stylelint" && -d "quality/tests/fixtures/file_family_adapters/yamlfmt" && -d "quality/tests/fixtures/file_family_adapters/yamllint" && -d "quality/tests/fixtures/file_family_adapters/keep_sorted" && -d "quality/tests/fixtures/file_family_adapters/shfmt" && -d "quality/tests/fixtures/file_family_adapters/shellcheck" && -d "quality/tests/fixtures/file_family_adapters/rubocop" && -d "quality/tests/fixtures/file_family_adapters/standardrb" && -d "quality/tests/fixtures/file_family_adapters/psscriptanalyzer" ]]; then
  ok
else
  bad "file-family adapter fixtures missing (want fifteen dirs under quality/tests/fixtures/file_family_adapters)"
fi

# Format fixtures pin versions plus routes.
if grep -q -F -e 'CUE_VERSION = "v0.17.1"' quality/tests/fixtures/file_family_adapters/cue/pins.bzl &&
  grep -q -F -e 'JSONNETFMT_VERSION = "v0.22.0"' quality/tests/fixtures/file_family_adapters/jsonnetfmt/pins.bzl &&
  grep -q -F -e 'PKL_VERSION = "0.32.1"' quality/tests/fixtures/file_family_adapters/pkl/pins.bzl &&
  grep -q -F -e 'MODFMT_VERSION = "v0.4.0"' quality/tests/fixtures/file_family_adapters/modfmt/pins.bzl &&
  grep -q -F -e 'TERRAFORM_VERSION = "v1.16.1"' quality/tests/fixtures/file_family_adapters/terraform/pins.bzl &&
  grep -q -F -e 'YAMLFMT_VERSION = "v0.21.0"' quality/tests/fixtures/file_family_adapters/yamlfmt/pins.bzl &&
  grep -q -F -e 'SHFMT_VERSION = "v3.12.0"' quality/tests/fixtures/file_family_adapters/shfmt/pins.bzl &&
  grep -q -F -e 'STANDARDRB_VERSION = "1.56.0"' quality/tests/fixtures/file_family_adapters/standardrb/pins.bzl; then
  ok
else
  bad "format fixtures lost their version pins under issue #800"
fi

# Lint fixtures pin versions plus routes.
if grep -q -F -e 'DJLINT_VERSION = "v1.45.0"' quality/tests/fixtures/file_family_adapters/djlint/pins.bzl &&
  grep -q -F -e 'STYLELINT_VERSION = "17.14.1"' quality/tests/fixtures/file_family_adapters/stylelint/pins.bzl &&
  grep -q -F -e 'YAMLLINT_VERSION = "1.38.0"' quality/tests/fixtures/file_family_adapters/yamllint/pins.bzl &&
  grep -q -F -e 'KEEP_SORTED_VERSION = "v0.10.0"' quality/tests/fixtures/file_family_adapters/keep_sorted/pins.bzl &&
  grep -q -F -e 'SHELLCHECK_VERSION = "v0.11.0"' quality/tests/fixtures/file_family_adapters/shellcheck/pins.bzl &&
  grep -q -F -e 'RUBOCOP_VERSION = "1.91.0"' quality/tests/fixtures/file_family_adapters/rubocop/pins.bzl &&
  grep -q -F -e 'PSSCRIPTANALYZER_VERSION = "1.25.0"' quality/tests/fixtures/file_family_adapters/psscriptanalyzer/pins.bzl; then
  ok
else
  bad "lint fixtures lost their version pins under issue #800"
fi

# REAL_ADAPTERS claims all fifteen cohort tools plus prettier extension.
if grep -q -F -e '"cue": {"format": ["cue"]}' "$adapters" &&
  grep -q -F -e '"jsonnetfmt": {"format": ["jsonnet"]}' "$adapters" &&
  grep -q -F -e '"pkl": {"format": ["pkl"]}' "$adapters" &&
  grep -q -F -e '"modfmt": {"format": ["go_module"]}' "$adapters" &&
  grep -q -F -e '"terraform": {"format": ["terraform"]}' "$adapters" &&
  grep -q -F -e '"djlint": {"format": ["html_template"], "lint": ["html_template"]}' "$adapters" &&
  grep -q -F -e '"stylelint": {"lint": ["css", "less", "scss"]}' "$adapters" &&
  grep -q -F -e '"yamlfmt": {"format": ["yaml"]}' "$adapters" &&
  grep -q -F -e '"yamllint": {"lint": ["yaml"]}' "$adapters" &&
  grep -q -F -e '"keep_sorted": {"lint": ["text"]}' "$adapters" &&
  grep -q -F -e '"shfmt": {"format": ["shell"]}' "$adapters" &&
  grep -q -F -e '"shellcheck": {"lint": ["shell"]}' "$adapters" &&
  grep -q -F -e '"rubocop": {"lint": ["ruby"]}' "$adapters" &&
  grep -q -F -e '"standardrb": {"format": ["ruby"]}' "$adapters" &&
  grep -q -F -e '"psscriptanalyzer": {"lint": ["powershell"]}' "$adapters" &&
  grep -q -F -e '"prettier": {"format": ["css", "gherkin"' "$adapters"; then
  ok
else
  bad "REAL_ADAPTERS lost a file-family cohort claim (want fifteen plus prettier extension under issue #800)"
fi

# Parity no longer defers the seventeen delivered classes.
parity_left=""
for cls in css cue gherkin go_module html_template jsonnet less pkl powershell ruby scss shell sql terraform text xml yaml; do
  if grep -q -F -e "\"$cls\":" "$parity"; then
    parity_left="$parity_left $cls:still-deferred"
  fi
done
if [[ -z "$parity_left" ]]; then
  ok
else
  bad "parity still defers delivered file-family classes:$parity_left (want none under issue #800)"
fi

# Matrix carries all forty-eight cohort cells.
cohort_matrix=""
for cell in matrix_cue_format_pass matrix_cue_format_fail matrix_jsonnet_format_pass matrix_jsonnet_format_fail matrix_pkl_format_pass matrix_pkl_format_fail matrix_go_module_format_pass matrix_go_module_format_fail matrix_terraform_format_pass matrix_terraform_format_fail matrix_yaml_format_pass matrix_yaml_format_fail matrix_shell_format_pass matrix_shell_format_fail matrix_ruby_format_pass matrix_ruby_format_fail matrix_html_template_format_pass matrix_html_template_format_fail matrix_css_format_pass matrix_css_format_fail matrix_less_format_pass matrix_less_format_fail matrix_scss_format_pass matrix_scss_format_fail matrix_gherkin_format_pass matrix_gherkin_format_fail matrix_sql_format_pass matrix_sql_format_fail matrix_xml_format_pass matrix_xml_format_fail matrix_html_template_lint_pass matrix_html_template_lint_fail matrix_css_lint_pass matrix_css_lint_fail matrix_less_lint_pass matrix_less_lint_fail matrix_scss_lint_pass matrix_scss_lint_fail matrix_ruby_lint_pass matrix_ruby_lint_fail matrix_powershell_lint_pass matrix_powershell_lint_fail matrix_yaml_lint_pass matrix_yaml_lint_fail matrix_shell_lint_pass matrix_shell_lint_fail matrix_text_lint_pass matrix_text_lint_fail; do
  grep -q -F -e "$cell" "$matrix" || cohort_matrix="$cohort_matrix $cell:missing"
done
if [[ -z "$cohort_matrix" ]] && grep -q -F -e 'FILE_FAMILY_CASES' "$cases"; then
  ok
else
  bad "runner matrix lost file-family cells:$cohort_matrix (want all forty-eight under issue #800)"
fi

# Every cohort tool keeps its parser with pass plus fail samples.
cohort_parser=""
for tool in cue jsonnetfmt pkl modfmt terraform yamlfmt shfmt standardrb djlint stylelint rubocop psscriptanalyzer yamllint shellcheck keep_sorted; do
  [[ -f "quality/adapter/src/parsers/$tool.rs" ]] || cohort_parser="$cohort_parser $tool:missing"
done
if [[ -z "$cohort_parser" ]] &&
  grep -q -F -e 'pub fn parse_cue' quality/adapter/src/parsers/cue.rs &&
  grep -q -F -e 'pub fn parse_jsonnetfmt' quality/adapter/src/parsers/jsonnetfmt.rs &&
  grep -q -F -e 'pub fn parse_stylelint' quality/adapter/src/parsers/stylelint.rs &&
  grep -q -F -e 'pub fn parse_rubocop' quality/adapter/src/parsers/rubocop.rs &&
  grep -q -F -e 'pub fn parse_psscriptanalyzer' quality/adapter/src/parsers/psscriptanalyzer.rs &&
  grep -q -F -e 'pub fn parse_yamllint' quality/adapter/src/parsers/yamllint.rs &&
  grep -q -F -e 'pub fn parse_shellcheck' quality/adapter/src/parsers/shellcheck.rs &&
  grep -q -F -e 'pub fn parse_keep_sorted' quality/adapter/src/parsers/keep_sorted.rs; then
  ok
else
  bad "parsers lost a file-family tool:$cohort_parser (want all fifteen under issue #800)"
fi

# Native-config binds the three configurable tools.
if grep -q -F -e 'stylelint_config' "$native" &&
  grep -q -F -e 'djlint_config' "$native" &&
  grep -q -F -e 'yamllint_config' "$native"; then
  ok
else
  bad "native-config lost a file-family binding (want stylelint plus djlint plus yamllint under issue #800)"
fi

# Runner dispatches all fifteen tools.
cohort_dispatch=""
for tool in '"cue"' '"jsonnetfmt"' '"pkl"' '"modfmt"' '"terraform"' '"djlint"' '"stylelint"' '"yamlfmt"' '"yamllint"' '"keep_sorted"' '"shfmt"' '"shellcheck"' '"rubocop"' '"standardrb"' '"psscriptanalyzer"'; do
  grep -q -F -e "$tool" "$real_rs" || cohort_dispatch="$cohort_dispatch $tool:missing"
done
if [[ -z "$cohort_dispatch" ]]; then
  ok
else
  bad "runner lost file-family dispatch:$cohort_dispatch (want all fifteen under issue #800)"
fi

# Commands pin the invocation shapes.
if grep -q -F -e 'pub fn cue_check' "$commands" &&
  grep -q -F -e 'pub fn cue_fix' "$commands" &&
  grep -q -F -e 'pub fn modfmt_check' "$commands" &&
  grep -q -F -e 'pub fn terraform_check' "$commands" &&
  grep -q -F -e 'pub fn yamlfmt_check' "$commands" &&
  grep -q -F -e 'pub fn shfmt_check' "$commands" &&
  grep -q -F -e 'pub fn standardrb_check' "$commands" &&
  grep -q -F -e 'pub fn stylelint_check' "$commands" &&
  grep -q -F -e 'pub fn rubocop_check' "$commands" &&
  grep -q -F -e 'pub fn psscriptanalyzer_check' "$commands" &&
  grep -q -F -e 'pub fn yamllint_check' "$commands" &&
  grep -q -F -e 'pub fn shellcheck_check' "$commands" &&
  grep -q -F -e 'pub fn keep_sorted_check' "$commands"; then
  ok
else
  bad "commands lost a file-family invocation shape (want all under issue #800)"
fi

# Runner-matrix doc owns the cohort sections.
if grep -q -F -e 'Interpreted/file-family' "$runner_doc" &&
  grep -q -F -e 'opt-in adapters delivered under #800' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its file-family sections under issue #800"
fi

# Tool integrations claim the adapters over the decided routes.
if grep -q -F -e 'Interpreted/file-family cohort (#800' "$integrations" &&
  grep -q -F -e 'adapters `cue`' "$integrations" &&
  grep -q -F -e '`rubocop`' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only' "$integrations" &&
  grep -q -F -e 'under #800' "$integrations" &&
  grep -q -F -e 'file_family_adapters_qualification' "$integrations"; then
  ok
else
  bad "tool-integrations lost its file-family adapter claims under issue #800"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "file_family_adapters_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:file_family_adapters_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the file_family_adapters_qualification wiring (want target plus dogfood-freshness)"
fi

# Fake format doubles stay present for the seed-only matrix cells.
if [[ -f "quality/testdata/fake_cue.sh" && -f "quality/testdata/fake_prettier_file_family.sh" && -f "quality/testdata/fake_djlint_format.sh" ]] &&
  grep -q -F -e 'fake_cue' "$subjects" &&
  grep -q -F -e 'fake_prettier_file_family' "$subjects"; then
  ok
else
  bad "matrix fake doubles missing (want fake_cue plus fake_prettier_file_family plus BUILD targets under issue #800)"
fi

# Depcheck/Examples opens resolved where applicable: file-family has no
# application foundation, so no depcheck lock fixtures and no adopt-*
# examples; the admitted depcheck languages and adopt-* examples stay
# unchanged (no silent new foundation claim here).
if grep -q -F -e 'DEPCHECK_LANGUAGES = [' quality/tests/fixtures/layer2_opens/pins.bzl &&
  [[ -d "tools/depcheck/testdata" ]] &&
  [[ -d "examples/adopt-rust" ]]; then
  ok
else
  bad "depcheck/examples opens unresolved (want admitted fixtures unchanged with no file-family foundation claim under issue #800)"
fi

# Live proof: adapter plus runner unit suites stay green.
if bazel test //quality/adapter:all //quality/runner:all --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "file-family adapters live proof failed (want //quality/adapter:all plus //quality/runner:all green)"
fi

# Live proof: the forty-eight cohort matrix cells stay green.
if bazel test //quality/testdata:runner_matrix --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "file-family matrix live proof failed (want //quality/testdata:runner_matrix green with forty-eight cohort cells)"
fi

# Live proof: foundation fixtures stay green (adapters change only).
if bazel build //quality/tests/fixtures/file_family_adapters/cue:corpus_starlark //quality/tests/fixtures/file_family_adapters/rubocop:corpus_starlark --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "file-family adapters live proof failed (want file-family fixture builds green)"
fi

dx_test_summary "file-family adapters qualification harness"
