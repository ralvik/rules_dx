#!/usr/bin/env bash
# Clap-as-tokenizer legacy-error qualification harness (issue #316).
#
# Decision: freeze legacy output as contract with snapshots (not strict clap
# parsing with auto help). Thin shims are Bazel action entrypoints with
# byte-identical diagnostics: env, codegen shard, env shard, evaluator,
# runner, markdown, plus the dx CLI tokenizer use clap derive only to map
# errors back to legacy hand-loop strings (unknown-flag phrasing,
# hyphen-value consumption, attached `=value` echo, value-parser
# rejections). Strict clap migration would change error text, help routing,
# and hyphen-value handling and break callers, so it stays owned gap.
#
# Qualifies the as-built frozen record with fixture evidence and owned gaps:
# - frozen with fixture evidence: `disable_help_flag` plus
#   `allow_hyphen_values` plus `invalid_token`/`parse_error` mapping per
#   shim, markdown snapshot tests pinning unknown/missing/malformed plus
#   hyphen-value plus attached-echo shapes, dx CLI recover/leading-flag
#   mapping onto UnknownOption/MissingValue/UnknownCommand;
# - stays frozen with owned reasons: unconditional next-token consumption
#   (even `--`-led), whole-token echo for attached `=value`, bare-flag
#   missing-value naming, value-parser rejections via the same legacy
#   parser;
# - open owned gap: strict clap parsing with auto help plus any future
#   migration.
#
# Versioned here, run by CI via `bazel run //tools/ci:clap_tokenizer_qualification`,
# following //tools/ci:helper_qualification.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

contract="docs/cli/cli-contract.md"
verify="docs/testing/verification-matrix.md"

# Contract owns the qualified seed-only record under #316.
if grep -q -F -e 'qualified seed-only under issue #316' "$contract" \
  && grep -q -F -e 'bazel run //tools/ci:clap_tokenizer_qualification' "$contract" \
  && grep -q -F -e 'frozen' "$contract" \
  && grep -q -F -e 'stays owned gap' "$contract"; then
  ok
else
  bad "cli-contract lost its qualified seed-only record under #316"
fi

# Contract keeps the freeze decision plus the owned-gap migration phrase
# (the latter also keeps //tools/ci:quality_foundations_guards green).
if grep -q -F -e 'freeze legacy' "$contract" \
  && grep -q -F -e 'output as contract' "$contract" \
  && grep -q -F -e 'migrate to strict' "$contract" \
  && grep -q -F -e 'allow_hyphen_values' "$contract"; then
  ok
else
  bad "cli-contract lost its freeze-decision plus migrate-to-strict owned-gap record"
fi

# Contract names every frozen site (issue names three, harness owns all seven).
if grep -q -F -e 'env' "$contract" \
  && grep -q -F -e 'codegen shard' "$contract" \
  && grep -q -F -e 'markdown' "$contract" \
  && grep -q -F -e 'env shard' "$contract" \
  && grep -q -F -e 'evaluator' "$contract" \
  && grep -q -F -e 'runner' "$contract" \
  && grep -q -F -e 'dx CLI tokenizer' "$contract"; then
  ok
else
  bad "cli-contract lost a frozen tokenizer site name"
fi

# Every tokenizer site carries a #316 frozen-contract decision record.
missing=""
for f in cli/env/src/main.rs generation/codegen_shard/src/main.rs env/env_shard/src/main.rs quality/evaluator/src/main.rs quality/runner/src/main.rs quality/markdown/src/lib.rs cli/cli/src/args/tokenizer.rs; do
  if ! grep -q -F -e '#316' "$f"; then
    missing="$missing $f"
  fi
done
if [[ -z "$missing" ]]; then
  ok
else
  bad "tokenizer sites missing a #316 decision record:$missing"
fi

# Every thin shim pins disable_help_flag (legacy help/usage routing, no auto help).
if grep -q -F -e 'disable_help_flag' cli/env/src/main.rs \
  && grep -q -F -e 'disable_help_flag' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'disable_help_flag' env/env_shard/src/main.rs \
  && grep -q -F -e 'disable_help_flag' quality/evaluator/src/main.rs \
  && grep -q -F -e 'disable_help_flag' quality/runner/src/main.rs \
  && grep -q -F -e 'disable_help_flag' quality/markdown/src/lib.rs; then
  ok
else
  bad "a thin shim lost its disable_help_flag legacy-help pin"
fi

# Every thin shim pins allow_hyphen_values (legacy unconditional next-token consumption).
if grep -q -F -e 'allow_hyphen_values' cli/env/src/main.rs \
  && grep -q -F -e 'allow_hyphen_values' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'allow_hyphen_values' env/env_shard/src/main.rs \
  && grep -q -F -e 'allow_hyphen_values' quality/evaluator/src/main.rs \
  && grep -q -F -e 'allow_hyphen_values' quality/runner/src/main.rs \
  && grep -q -F -e 'allow_hyphen_values' quality/markdown/src/lib.rs; then
  ok
else
  bad "a thin shim lost its allow_hyphen_values hyphen-consumption pin"
fi

# Every site keeps the invalid_token plus parse-error mapping entrypoint.
if grep -q -F -e 'fn invalid_token' cli/env/src/main.rs \
  && grep -q -F -e 'fn parse_error' cli/env/src/main.rs \
  && grep -q -F -e 'fn invalid_token' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'fn parse_error' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'fn invalid_token' env/env_shard/src/main.rs \
  && grep -q -F -e 'fn parse_error' env/env_shard/src/main.rs \
  && grep -q -F -e 'fn invalid_token' quality/evaluator/src/main.rs \
  && grep -q -F -e 'fn parse_error' quality/evaluator/src/main.rs \
  && grep -q -F -e 'fn invalid_token' quality/runner/src/main.rs \
  && grep -q -F -e 'fn parse_error' quality/runner/src/main.rs \
  && grep -q -F -e 'fn invalid_token' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn parse_error' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn invalid_token' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'fn map_clap_error' cli/cli/src/args/tokenizer.rs; then
  ok
else
  bad "a tokenizer site lost its invalid_token plus parse-error mapping"
fi

# Legacy unknown-flag phrasing stays frozen per family.
if grep -q -F -e 'unknown flag' cli/env/src/main.rs \
  && grep -q -F -e 'unknown flag' quality/evaluator/src/main.rs \
  && grep -q -F -e 'unknown flag' quality/runner/src/main.rs \
  && grep -q -F -e 'unknown argument' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'unknown argument' env/env_shard/src/main.rs \
  && grep -q -F -e 'unknown argument' quality/markdown/src/lib.rs; then
  ok
else
  bad "a site lost its frozen unknown-flag/unknown-argument phrasing"
fi

# Legacy missing-value phrasing stays frozen where the legacy loop named the
# bare flag (env, evaluator, runner, markdown, dx CLI). Shard writers report
# a bare usage line here instead (checked below), matching their legacy loop.
if grep -q -F -e 'missing value for' cli/env/src/main.rs \
  && grep -q -F -e 'missing value for' quality/evaluator/src/main.rs \
  && grep -q -F -e 'missing value for' quality/runner/src/main.rs \
  && grep -q -F -e 'missing value for' quality/markdown/src/lib.rs \
  && grep -q -F -e 'MissingValue' cli/cli/src/args/error.rs \
  && grep -q -F -e 'ErrorKind::InvalidValue => usage()' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'ErrorKind::InvalidValue => usage()' env/env_shard/src/main.rs; then
  ok
else
  bad "a site lost its frozen missing-value phrasing"
fi

# Shard-writer value rejections stay frozen as bad-entry text.
if grep -q -F -e 'bad --entry' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'bad --entry' env/env_shard/src/main.rs; then
  ok
else
  bad "a shard writer lost its frozen bad---entry rejection text"
fi

# Evaluator threshold plus runner/markdown mapping rejections stay frozen.
if grep -q -F -e 'unknown fail_on' quality/evaluator/src/main.rs \
  && grep -q -F -e 'malformed --stage' quality/runner/src/main.rs \
  && grep -q -F -e 'malformed --source' quality/markdown/src/lib.rs \
  && grep -q -F -e 'malformed --sibling' quality/markdown/src/lib.rs; then
  ok
else
  bad "evaluator/runner/markdown lost a frozen value-rejection pin"
fi

# Attached `=value` whole-token echo stays documented at every shim mapping.
if grep -q -F -e 'strips an attached' cli/env/src/main.rs \
  && grep -q -F -e 'strips an attached' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'strips an attached' env/env_shard/src/main.rs \
  && grep -q -F -e 'strips an attached' quality/evaluator/src/main.rs \
  && grep -q -F -e 'strips an attached' quality/runner/src/main.rs \
  && grep -q -F -e 'strips an attached' quality/markdown/src/lib.rs; then
  ok
else
  bad "a site lost its attached-=value echo recovery pin"
fi

# Hyphen-value consumption stays documented as legacy parity: every value
# option consumes the next token unconditionally, even a `--`-led one.
if grep -q -F -e 'unconditional' cli/env/src/main.rs \
  && grep -q -F -e 'unconditional' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'unconditional' env/env_shard/src/main.rs \
  && grep -q -F -e 'unconditional' quality/evaluator/src/main.rs \
  && grep -q -F -e 'unconditional' quality/runner/src/main.rs \
  && grep -q -F -e 'unconditional' quality/markdown/src/lib.rs; then
  ok
else
  bad "a site lost its hyphen-value legacy-parity record"
fi

# dx CLI tokenizer keeps the verbatim-tail plus suggestion mapping contract.
if grep -q -F -e 'split_bazel_verbatim' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'recover_token' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'leading_flag' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'UnknownOption' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'MissingValue' cli/cli/src/args/tokenizer.rs \
  && grep -q -F -e 'UnknownCommand' cli/cli/src/args/tokenizer.rs; then
  ok
else
  bad "dx CLI tokenizer lost its verbatim-tail plus error-mapping pins"
fi

# Fixture evidence: markdown snapshot tests pin unknown/missing/malformed shapes.
if grep -q -F -e 'fn cli_unknown_argument_fails' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn cli_missing_value_fails' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn cli_malformed_mapping_fails' quality/markdown/src/lib.rs \
  && grep -q -F -e 'unknown argument: --bogus' quality/markdown/src/lib.rs \
  && grep -q -F -e 'missing value for --source' quality/markdown/src/lib.rs \
  && grep -q -F -e 'malformed --source' quality/markdown/src/lib.rs; then
  ok
else
  bad "markdown lost its unknown/missing/malformed snapshot tests"
fi

# Fixture evidence: hyphen-value plus attached-echo plus positional plus empty shapes.
if grep -q -F -e 'fn cli_flag_as_value_is_malformed' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn cli_attached_forms_echo_whole_token' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn cli_bare_positional_is_unknown_argument' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn cli_empty_value_is_malformed' quality/markdown/src/lib.rs \
  && grep -q -F -e '--sibling' quality/markdown/src/lib.rs \
  && grep -q -F -e '--source=' quality/markdown/src/lib.rs; then
  ok
else
  bad "markdown lost its hyphen-value/attached/positional/empty snapshot tests"
fi

# Fixture evidence: usage line rides every markdown failure (exit 2 surface).
if grep -q -F -e 'starts_with("usage: quality_markdown")' quality/markdown/src/lib.rs \
  && grep -q -F -e 'fn usage' quality/markdown/src/lib.rs; then
  ok
else
  bad "markdown lost its usage-line failure-surface pin"
fi

# Fixture evidence: shard-writer usage routing plus evaluator threshold parser.
if grep -q -F -e 'fn usage' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'fn usage' env/env_shard/src/main.rs \
  && grep -q -F -e 'fn parse_fail_on' quality/evaluator/src/main.rs \
  && grep -q -F -e 'fn parse_entry_value' generation/codegen_shard/src/main.rs \
  && grep -q -F -e 'fn parse_entry_value' env/env_shard/src/main.rs; then
  ok
else
  bad "shard writers/evaluator lost a usage-routing or value-parser pin"
fi

# Verification matrix keeps the #316 qualified record with owned gaps.
if grep -q -F -e 'clap_tokenizer_qualification' "$verify" \
  && grep -q -F -e 'qualified seed-only under #316' "$verify" \
  && grep -q -F -e 'strict clap' "$verify"; then
  ok
else
  bad "verification-matrix lost its #316 clap-tokenizer qualification record"
fi

echo "clap-tokenizer qualification harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
