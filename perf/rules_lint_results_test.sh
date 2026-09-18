#!/usr/bin/env bash
# Rules-lint measured-results validation (issue #86).
#
# Validates the checked-in seed-host measured report
# (perf/rules_lint_results.json): JSON parses, synth-tree digest matches a
# fresh harness regeneration (determinism), fairness pins recorded,
# quality-sample aquery count present, timing fields positive, and the
# report stays report-not-gate (gate false, claim none). Tagged
# `no-coverage`: timing data stays out of the coverage denominator.
set -euo pipefail

results="$1"
harness="$2"
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0

check_json() { # name python-expr
  local name="$1" expr="$2"
  if python3 -c "import json,sys; doc=json.load(open(sys.argv[1])); assert ($expr), 'check failed'" "$results"; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name" >&2
    fail=$((fail + 1))
  fi
}

check_json "synth files" "doc['seed_harness']['files']==200"
check_json "synth dirty" "doc['seed_harness']['dirty_files']==20"
check_json "synth dirty_pct" "doc['seed_harness']['dirty_pct']==10"
check_json "synth seed" "doc['seed_harness']['seed']==86"
check_json "rules_lint pin" "doc['rules_lint_pin']=='v2.8.0'"
check_json "host" "doc['host']=='linux_x86_64'"
check_json "bazel version" "doc['bazel_version']=='9.2.0'"
check_json "seed generation_ms positive" "doc['seed_harness']['generation_ms']>0"
check_json "tree digest 64-hex" "len(doc['seed_harness']['tree_sha256'])==64 and all(c in '0123456789abcdef' for c in doc['seed_harness']['tree_sha256'])"
check_json "report-not-gate" "doc['gate'] is False and doc['claim']=='none'"
check_json "quality sample actions" "doc['quality_sample']['aquery_actions']==2"
check_json "quality sample timing positive" "doc['quality_sample']['warm_wall_ms']>0 and doc['quality_sample']['no_cache_wall_ms']>0"
check_json "quality mnemonics" "'DxRealQualityLint' in doc['quality_sample']['mnemonics'] and 'DxRealQualityFormat' in doc['quality_sample']['mnemonics']"
check_json "runner timing positive" "doc['runner_test_timing']['cold_ish_wall_ms']>0 and doc['runner_test_timing']['warm_wall_ms']>0"
check_json "runner aquery" "doc['runner_test_timing']['aquery_actions']==6"
check_json "peak-memory gap explicit" "doc['peak_memory']['collected'] is False"
check_json "post-shutdown cold measured" "doc['quality_sample']['post_shutdown_cold_wall_ms']>0 and doc['quality_sample']['warm_after_cold_wall_ms']>0 and doc['quality_sample']['post_shutdown_cold_wall_ms']>doc['quality_sample']['warm_after_cold_wall_ms']"
check_json "post-shutdown cold evidence" "'bazel shutdown' in doc['quality_sample']['post_shutdown_cold_command'] and '1615' in doc['quality_sample']['post_shutdown_cold_evidence']"
check_json "no-cache flag explicit" "'--nouse_action_cache' in doc['quality_sample']['no_cache_flag'] and 'server warm' in doc['quality_sample']['no_cache_flag']"
check_json "rules_lint-side gap explicit" "'rules_lint-side' in doc['note']"

# Determinism: fresh harness regeneration matches the checked-in digest.
bash "$harness" --files 200 --dirty-pct 10 --seed 86 --out "$scratch/regen" > "$scratch/regen.json"
d_regen="$(python3 -c 'import json; print(json.load(open("'"$scratch"'/regen.json"))["tree_sha256"])')"
d_checked="$(python3 -c 'import json; print(json.load(open("'"$results"'"))["seed_harness"]["tree_sha256"])')"
if [[ "$d_regen" == "$d_checked" && -n "$d_regen" ]]; then pass=$((pass + 1)); else echo "FAIL: regen digest ($d_regen vs $d_checked)" >&2; fail=$((fail + 1)); fi

echo "rules_lint_results_test: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
