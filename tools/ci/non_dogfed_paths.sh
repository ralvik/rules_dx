#!/usr/bin/env bash
# Non-dogfed execution plan (issue #324).
#
# Four cohorts never run under the standard dogfood/CI gates by design
# (manual tags, carve-outs, tag suppression). Each has an explicit
# execution path, never a silent gap:
#
# - integration/ E2E drivers: .bazelignore'd out of the parent universe,
#   carved out of both ownership audits, `manual` (+`exclusive`, `local`,
#   `no-sandbox`) drivers in the explicit `:e2e` suite, orphan-guarded by
#   `e2e_cases`, enumerated by `manual_negatives`, run only via the
#   explicit `E2E_WORKSPACE=... bazel test //tools/ci:e2e` suite in CI's
#   `e2e` job. Child POSIX fixtures (`pass.sh`/`fail.sh`) arrive by shell
#   copy and execute as child `sh_test` inside the staged workspace.
# - negative fixtures: the four `libs/starlark/tests/negative` demos plus
#   the markdown-no-config subject carry `manual`, so wildcard suites skip
#   them; `manual_negatives` runs each explicitly and asserts it still
#   fails for its documented reason, with an enumeration guard so a new
#   manual test forces classification. Run in CI's `prove` job.
# - no-coverage cohort: `no-coverage` tests run under `bazel test //...`
#   (never `manual`) and skip only under `bazel coverage` via the
#   `test_tag_filters=-no-coverage` preset; `target_tags` proves the skip
#   semantics on the representative pair, `coverage_cell`/`coverage_spill`/
#   `coverage_qualification` own the gate halves. Run in CI's `test` job
#   (execution) plus `coverage`/`prove` jobs (exclusion proof).
# - shell sources with no quality class: `shell` is a known semantic class
#   but `real_source_target` owns no shell sources (no adapter), and both
#   ownership audits intentionally exclude `.sh` (corpus covers only
#   BUILD/MODULE/bzl/toml/md; code ownership covers only code languages).
#   Every checked-in `.sh` rides `deps(//...)` except the two
#   integration/ POSIX fixtures above; execution is `bazel test //...`
#   for sh_test/sh_binary plus the explicit `:e2e` suite for manual
#   drivers, portability via `shell_contract`. POSIX fixtures stay portable
#   with no Linux constraint.
#
# This harness machine-checks the plan statically on a clean tree (plus
# one deps-closure query for shell ownership, mirroring the ownership
# audits). Versioned here, run by CI via
# `bazel run //tools/ci:non_dogfed_paths`, following //tools/ci:code_ownership.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_mkscratch scratch

# --- A. integration/ E2E explicit suite ---

# A1: integration/ stays out of the parent universe.
if grep -q -F -e 'integration/' .bazelignore; then
  ok
else
  bad ".bazelignore lost the integration/ carve-out (E2E scenarios must stay out of //...)"
fi

# A2: both ownership audits carve out integration/ with marker checks.
if grep -q -F -e 'integration/' tools/ci/corpus_audit.sh &&
  grep -q -F -e "grep -v -E '^integration/'" tools/ci/corpus_audit.sh; then
  ok
else
  bad "corpus_audit lost its integration/ carve-out"
fi

if grep -q -F -e 'integration/' tools/ci/code_ownership.sh &&
  grep -q -F -e "grep -v -E '^integration/'" tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership lost its integration/ carve-out"
fi

# A3: E2E drivers are manual + exclusive + local + no-sandbox.
for driver in e2e_clean e2e_dirty e2e_format_roundtrip; do
  build="$(bazel query --output=build "//tools/ci:$driver" 2>/dev/null)"
  if echo "$build" | grep -q -F -e '"manual"' &&
    echo "$build" | grep -q -F -e '"exclusive"' &&
    echo "$build" | grep -q -F -e '"local"' &&
    echo "$build" | grep -q -F -e '"no-sandbox"'; then
    ok
  else
    bad "driver //tools/ci:$driver lost its manual+exclusive+local+no-sandbox tags"
  fi
done

# A4: explicit :e2e suite holds exactly the three drivers.
suite_tests="$(bazel query 'tests(//tools/ci:e2e)' 2>/dev/null | LC_ALL=C sort -u)"
if echo "$suite_tests" | grep -q -F -e '//tools/ci:e2e_clean' &&
  echo "$suite_tests" | grep -q -F -e '//tools/ci:e2e_dirty' &&
  echo "$suite_tests" | grep -q -F -e '//tools/ci:e2e_format_roundtrip' &&
  [[ "$(echo "$suite_tests" | wc -l | tr -d ' ')" == "3" ]]; then
  ok
else
  bad ":e2e suite lost a driver or gained an unreviewed member: $suite_tests"
fi

# A5: every integration scenario is wired (no orphan silently never runs).
orphan=0
for mod in integration/*/MODULE.bazel; do
  name="$(basename "$(dirname "$mod")")"
  if grep -rq -F -e "$name" tools/ci/e2e.sh tools/ci/e2e_format.sh tools/ci/BUILD.bazel; then
    ok
  else
    bad "integration/$name/ is orphaned: no reference in e2e.sh, e2e_format.sh, or BUILD.bazel"
    orphan=1
  fi
done
if [[ "$orphan" == "0" ]] && [[ ! -f integration/README.md ]]; then
  bad "integration/README.md missing (E2E-case convention owner)"
fi

# A6: CI runs the convention guard plus the explicit suite (never //...).
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:e2e_cases' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel test --noshow_progress //tools/ci:e2e' .github/workflows/ci.yml &&
  grep -q -F -e 'E2E_WORKSPACE=$GITHUB_WORKSPACE' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the e2e job explicit-suite wiring (e2e_cases + //tools/ci:e2e with E2E_WORKSPACE)"
fi

# A7: README documents explicit-only invocation.
if grep -q -F -e 'E2E_WORKSPACE=$PWD bazel test //tools/ci:e2e' integration/README.md &&
  grep -q -F -e 'manual' integration/README.md; then
  ok
else
  bad "integration/README.md lost the explicit-only invocation record"
fi

# A8: child POSIX fixtures execute as child sh_test, staged by shell copy.
if grep -q -F -e 'name = "pass"' integration/clean/BUILD.bazel &&
  grep -q -F -e 'pass.sh' integration/clean/BUILD.bazel &&
  grep -q -F -e 'name = "fail"' integration/dirty/BUILD.bazel &&
  grep -q -F -e 'fail.sh' integration/dirty/BUILD.bazel; then
  ok
else
  bad "integration child fixtures lost their pass/fail sh_test wiring"
fi

if grep -q -F -e 'cp -r "$workspace/integration/$scenario/." "$scratch/child/"' tools/ci/e2e.sh &&
  grep -q -F -e 'RULES_DX_ROOT' tools/ci/e2e.sh; then
  ok
else
  bad "e2e.sh lost its shell-copy staging (scenario files must arrive by copy, never by label)"
fi

# A9: manual_negatives enumerates the E2E drivers so a new manual test forces classification.
if grep -q -F -e '//tools/ci:e2e_clean' tools/ci/manual_negatives.sh &&
  grep -q -F -e '//tools/ci:e2e_dirty' tools/ci/manual_negatives.sh &&
  grep -q -F -e '//tools/ci:e2e_format_roundtrip' tools/ci/manual_negatives.sh &&
  grep -q -F -e "attr(tags, manual, kind(test, //...))" tools/ci/manual_negatives.sh; then
  ok
else
  bad "manual_negatives lost its E2E-driver enumeration guard"
fi

# --- B. negative fixtures (manual expected-fail demos) ---

# B1: four starlark demos stay manual.
demos="$(bazel query 'attr(tags, manual, //libs/starlark/tests/negative/...)' 2>/dev/null | LC_ALL=C sort -u)"
if echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:failing_check_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:missing_observation_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:missing_fragment_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:wrong_phase_demo'; then
  ok
else
  bad "starlark negative demos lost manual tags: $demos"
fi

# B2: markdown-no-config subject stays manual.
if bazel query 'attr(tags, manual, //quality/testdata:fixture_real_markdown_no_config_subject)' 2>/dev/null | grep -q -F -e 'fixture_real_markdown_no_config_subject'; then
  ok
else
  bad "markdown-no-config subject lost its manual tag (must never run under //...)"
fi

# B3: manual_negatives proves each demo still fails for its documented reason.
if grep -q -F -e 'failing_check_demo' tools/ci/manual_negatives.sh &&
  grep -q -F -e 'missing_observation_demo' tools/ci/manual_negatives.sh &&
  grep -q -F -e 'missing_fragment_demo' tools/ci/manual_negatives.sh &&
  grep -q -F -e 'wrong_phase_demo' tools/ci/manual_negatives.sh &&
  grep -q -F -e 'fixture_real_markdown_no_config_subject' tools/ci/manual_negatives.sh; then
  ok
else
  bad "manual_negatives lost an expected-failure proof (four demos + markdown-no-config)"
fi

# B4: CI prove job runs the negative proof.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:manual_negatives' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml prove job lost its manual_negatives step"
fi

# B5: starlark docs own the explicit-run record plus the CI pin.
if grep -q -F -e 'tags = ["manual"]' docs/testing/starlark.md &&
  grep -q -F -e 'bazel run //tools/ci:manual_negatives' docs/testing/starlark.md; then
  ok
else
  bad "docs/testing/starlark.md lost the manual-demo explicit-run plus CI-pin record"
fi

# --- C. no-coverage cohort (coverage-excluded runs) ---

# C1: coverage preset suppresses no-coverage tags only.
if grep -q -F -e 'coverage --test_tag_filters=-no-coverage' tools/bazelrc/preset.bazelrc; then
  ok
else
  bad "preset.bazelrc lost the -no-coverage coverage filter"
fi

# C2: no test is both manual and no-coverage (cohort runs under test, skips only under coverage).
both="$(bazel query 'attr(tags, manual, attr(tags, no-coverage, kind(test, //...)))' 2>/dev/null || true)"
if [[ -z "$both" ]]; then
  ok
else
  bad "a test carries both manual and no-coverage (must be exactly one exclusion): $both"
fi

# C3: the cohort is non-empty and every member is a test (fails closed on zero).
cohort_count="$(bazel query 'attr(tags, no-coverage, kind(test, //...))' 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$cohort_count" -ge 10 ]]; then
  ok
else
  bad "no-coverage cohort unexpectedly small (found $cohort_count, want >= 10)"
fi

# C4: matrix fail fixtures stay no-coverage but executable (representative negative coverage).
if grep -q -F -e 'tags = ["no-coverage"]' quality/testdata/runner_matrix_tests.bzl &&
  bazel query 'attr(tags, no-coverage, //quality/testdata/...)' 2>/dev/null | grep -q -F -e 'matrix_python_lint_fail'; then
  ok
else
  bad "quality matrix fail fixtures lost their no-coverage execution wiring"
fi

# C5: target_tags proves the skip semantics on the representative pair.
if grep -q -F -e 'hello_output_test' tools/ci/target_tags.sh &&
  grep -q -F -e 'hello_test' tools/ci/target_tags.sh &&
  grep -q -F -e 'no-coverage hello_output_test ran under bazel coverage' tools/ci/target_tags.sh; then
  ok
else
  bad "target_tags lost its no-coverage skip proof (hello_output_test vs hello_test)"
fi

# C6: gate halves stay wired in CI (execution in test, exclusion proof in coverage/prove).
if grep -q -F -e 'bazel test --noshow_progress //...' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:target_tags' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_cell' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_qualification' .github/workflows/ci.yml; then
  ok
else
  bad "CI lost a no-coverage gate half (test execution + target_tags + coverage_cell + coverage_qualification)"
fi

# C7: cells registry stays seed-only with no cross-cell union.
if grep -q -F -e 'qualified seed-linux_x86_64' tools/coverage/cells.txt &&
  grep -q -F -e 'tools/coverage/seed-inventory.txt' tools/coverage/cells.txt &&
  [[ -f tools/coverage/seed-inventory.txt ]]; then
  ok
else
  bad "coverage cells registry lost its seed-only qualified record"
fi

# --- D. shell sources with no quality class ---

# D1: shell is a known class but real_source_target owns no shell sources (no adapter by design).
if grep -q -F -e '"shell",' quality/sources.bzl &&
  ! grep -q -F -e 'shell_srcs' quality/fixtures.bzl; then
  ok
else
  bad "shell quality-class wiring broke (want known class in sources.bzl, no shell_srcs in fixtures.bzl)"
fi

# D2: corpus audit intentionally covers only BUILD/MODULE/bzl/toml/md (never .sh).
if grep -q -F -e "(BUILD\.bazel|MODULE\.bazel)" tools/ci/corpus_audit.sh &&
  grep -q -F -e "bzl|toml|md" tools/ci/corpus_audit.sh &&
  ! grep -E -e "grep -E.*\\\\\.sh" tools/ci/corpus_audit.sh | grep -q .; then
  ok
else
  bad "corpus_audit filter broke (want BUILD/MODULE/bzl/toml/md only, never .sh)"
fi

# D3: code ownership intentionally covers only code languages (never .sh).
if grep -q -F -e 'rs|py|js' tools/ci/code_ownership.sh &&
  ! grep -q -F -e '\.sh$' tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership filter broke (want code languages only, never .sh)"
fi

# D4: every checked-in .sh rides deps(//...) except the two integration POSIX fixtures.
git ls-files '*.sh' | LC_ALL=C sort -u >"$scratch/all_sh.txt"
bazel query "kind('source file', deps(//...))" 2>/dev/null |
  grep -E '^(@@)?//' |
  sed 's/^@@//; s|^//||; s|:|/|; s|^/||' |
  grep '\.sh$' |
  grep -v '^@' |
  LC_ALL=C sort -u >"$scratch/owned_sh_raw.txt"
comm -23 "$scratch/all_sh.txt" "$scratch/owned_sh_raw.txt" >"$scratch/unowned_sh.txt" || true
unowned_count="$(wc -l <"$scratch/unowned_sh.txt" | tr -d ' ')"
if [[ "$unowned_count" == "2" ]] &&
  grep -q -F -x -e 'integration/clean/pass.sh' "$scratch/unowned_sh.txt" &&
  grep -q -F -x -e 'integration/dirty/fail.sh' "$scratch/unowned_sh.txt"; then
  ok
else
  bad "shell ownership broke (want exactly the 2 integration POSIX fixtures unowned, found $unowned_count: $(tr '\n' ' ' <"$scratch/unowned_sh.txt"))"
fi

# D5: shell execution paths stay wired (test job for sh tests, explicit suite for manual drivers).
if grep -q -F -e 'bazel test --noshow_progress //...' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel test --noshow_progress //tools/ci:e2e' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_contract' .github/workflows/ci.yml; then
  ok
else
  bad "CI lost a shell execution path (test //... + explicit :e2e + shell_contract)"
fi

# D6: shell_contract owns portability (bash-only Linux harness, POSIX fixtures portable).
if grep -q -F -e '//tools/ci:shell_contract' docs/testing/tools.md &&
  grep -q -F -e 'is bash-only on the Linux seed host (decided' docs/testing/tools.md; then
  ok
else
  bad "docs/testing/tools.md lost the decided shell-contract record"
fi

# --- E. plan record (no silent gaps) ---

# E1: verification matrix owns the delivered execution-plan record (not a stay-open tracker).
if grep -q -F -e 'Non-dogfed execution plan' docs/testing/verification-matrix.md &&
  grep -q -F -e 'bazel run //tools/ci:non_dogfed_paths' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #324 delivered execution-plan record"
fi

# E2: this harness is wired in CI's dogfood-freshness job alongside the ownership audits.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:non_dogfed_paths' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the non_dogfed_paths step (want dogfood-freshness alongside corpus_audit/code_ownership)"
fi

dx_test_summary "non-dogfed paths harness"
