#!/usr/bin/env bash
# Non-dogfed execution plan (issues #324, #407).
#
# Cohorts that never run under the standard dogfood/CI gates by design
# (explicit suites, carve-outs, tag suppression). Each has an explicit
# execution path, never a silent gap:
#
# - CLI contract (issue #407, replaces nested E2E): no `integration/`
#   workspace, no second Bazel download, no `manual`/`local`/`exclusive`/
#   `no-sandbox`, no `long` timeouts. Equivalents run hermetically under
#   `bazel test //...`: `dx test`/`dx build` exit-code preservation via
#   `cli/cli/src/exec` unit pins (including Bazel test-failure code 3),
#   format rewrite `x=1` -> `x = 1` via `//quality/testdata:runner_matrix`
#   goldens, aspect wiring via `DxSubjectInfo` pins
#   (`//quality/testdata:real_aspect_*`), preset check→update→recheck via
#   `//:preset_parity_test`; `local_path_override` plus `dx_dev` wiring via
#   the adopt-rust smoke in normal CI. What is lost (real-daemon exit 3,
#   real Buildifier rewrite, full consumer wiring now smoke-only) is
#   recorded in `docs/testing/verification-matrix.md`.
# - negative fixtures: the four `libs/starlark/tests/negative` demos plus
#   the markdown-no-config subject are green hermetic proofs (issue #406):
#   execution failures as passing `sh_test` goldens, analysis failures via
#   `failure_test` (`analysistest.expect_failure`). No `manual`, no nested
#   Bazel; failure lives inside passing bodies. The red subjects stay
#   `manual` (non-test, so wildcard builds skip them) where the
#   analysistest contract requires it. Run in CI's `test` job via
#   `bazel test //...` (no separate prove step).
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
#   Every checked-in `.sh` rides `deps(//...)`; execution is
#   `bazel test //...` for sh_test/sh_binary, portability via
#   `shell_contract`. POSIX fixtures stay portable with no Linux constraint.
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

# --- A. CLI contract hermetic (issue #407, replaces nested E2E) ---

# A1: no integration/ workspace, no carve-out, no E2E drivers.
if [[ -d "integration" ]]; then
  bad "integration/ workspace still present (issue #407: nested E2E deleted)"
else
  ok
fi
if grep -q -F -e 'integration/' .bazelignore; then
  bad ".bazelignore still carries the integration/ carve-out (issue #407: deleted)"
else
  ok
fi
if [[ -f "tools/ci/e2e.sh" ]] || [[ -f "tools/ci/e2e_format.sh" ]] || [[ -f "tools/ci/e2e_preset.sh" ]] || [[ -f "tools/ci/e2e_cases.sh" ]] || [[ -f "integration/README.md" ]]; then
  bad "nested E2E driver files still present (e2e.sh/e2e_format.sh/e2e_preset.sh/e2e_cases.sh/integration/README.md, issue #407)"
else
  ok
fi
if bazel query '//tools/ci:e2e' 2>/dev/null | grep -q .; then
  bad "//tools/ci:e2e suite still present (issue #407: deleted)"
else
  ok
fi

# A2: both ownership audits carry no integration/ carve-out filter.
if grep -q -F -e "grep -v -E '^integration/'" tools/ci/corpus_audit.sh; then
  bad "corpus_audit still carries the integration/ carve-out filter (issue #407)"
else
  ok
fi
if grep -q -F -e "grep -v -E '^integration/'" tools/ci/code_ownership.sh; then
  bad "code_ownership still carries the integration/ carve-out filter (issue #407)"
else
  ok
fi

# A3: no manual/local/exclusive/no-sandbox tests; no long timeouts.
# Manual red subjects stay manual only as non-tests (issue #406); kind(test)
# must be empty for all four sandbox-escape tags.
for tag in manual local exclusive no-sandbox; do
  tagged="$(bazel query "attr(tags, $tag, kind(test, //...))" 2>/dev/null || true)"
  if [[ -z "$tagged" ]]; then
    ok
  else
    bad "manual/local/exclusive/no-sandbox test still present ($tag): $tagged (issue #407: 0x manual/local/exclusive/no-sandbox)"
  fi
done
if bazel query 'attr(timeout, long, kind(test, //...))' 2>/dev/null | grep -q .; then
  bad "a long-timeout test remains (issue #407: no long timeouts)"
else
  ok
fi

# A4: hermetic equivalents run under `bazel test //...`.
if [[ -f "cli/cli/src/exec/test_support.rs" ]] &&
  grep -q -F -e 'code' cli/cli/src/exec/bazel.rs &&
  grep -q -F -e 'Some(3)' cli/cli/src/exec/bazel.rs; then
  ok
else
  bad "exec argv/code hermetic pins missing (test_support.rs + bazel.rs code 3, issue #407)"
fi
if bazel query '//quality/testdata:runner_matrix' 2>/dev/null | grep -q -F -e 'runner_matrix' &&
  bazel query '//quality/testdata:real_aspect_presence' 2>/dev/null | grep -q -F -e 'real_aspect_presence' &&
  grep -rq -F -e 'DxSubjectInfo' quality/testdata/ quality/ 2>/dev/null; then
  ok
else
  bad "runner-matrix goldens / DxSubjectInfo analysis pins missing (issue #407)"
fi
if bazel query '//:preset_parity_test' 2>/dev/null | grep -q -F -e 'preset_parity_test'; then
  ok
else
  bad "preset check→update→recheck pin missing (//:preset_parity_test, issue #407)"
fi

# A5: MODULE carries no second-Bazel download.
if grep -q -F -e 'bazel_binaries.download' MODULE.bazel ||
  grep -q -F -e 'rules_bazel_integration_test' MODULE.bazel; then
  bad "MODULE.bazel still carries the second-Bazel pin (issue #407: single Bazel only)"
else
  ok
fi
if grep -q -F -e 'bazel_binaries' MODULE.bazel.lock; then
  bad "MODULE.bazel.lock still carries bazel_binaries (issue #407: regenerate without the dev-dep)"
else
  ok
fi

# A6: CI runs the adopt-rust dx_dev smoke in normal CI, never a nested E2E job.
if grep -q -F -e 'E2E_WORKSPACE' .github/workflows/ci.yml ||
  grep -q -F -e '//tools/ci:e2e' .github/workflows/ci.yml; then
  bad "ci.yml still wires the nested E2E suite (E2E_WORKSPACE / //tools/ci:e2e, issue #407)"
else
  ok
fi
if grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the adopt-rust dx_dev smoke (local_path_override + dx_dev wiring, issue #407)"
fi

# A7: loss record lives in the verification matrix.
if grep -q -F -e 'real-daemon exit 3' docs/testing/verification-matrix.md &&
  grep -q -F -e 'real Buildifier rewrite' docs/testing/verification-matrix.md &&
  grep -q -F -e 'smoke-only' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost the #407 loss record (real-daemon exit 3, real Buildifier rewrite, smoke-only)"
fi

# --- B. negative fixtures (green hermetic proofs, issue #406) ---

# B1: four starlark demos are green (non-manual) passing tests.
demos="$(bazel query '//libs/starlark/tests/negative/...' 2>/dev/null | LC_ALL=C sort -u)"
if echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:failing_check_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:missing_observation_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:missing_fragment_demo' &&
  echo "$demos" | grep -q -F -e '//libs/starlark/tests/negative:wrong_phase_demo'; then
  ok
else
  bad "starlark negative proofs missing: $demos"
fi
if bazel query 'attr(tags, manual, kind(test, //libs/starlark/tests/negative/...))' 2>/dev/null | grep -q .; then
  bad "starlark negative proofs must not be manual (green hermetic, issue #406)"
else
  ok
fi

# B2: markdown-no-config subject stays manual.
if bazel query 'attr(tags, manual, //quality/testdata:fixture_real_markdown_no_config_subject)' 2>/dev/null | grep -q -F -e 'fixture_real_markdown_no_config_subject'; then
  ok
else
  bad "markdown-no-config subject lost its manual tag (must never run under //...)"
fi

# B3: green proofs replace the manual_negatives shell loop (deleted per
# #406): execution failures via sh_test goldens, analysis via failure_test.
if grep -q -F -e 'failure_test' libs/starlark/tests/negative/negative_tests.bzl &&
  grep -q -F -e 'failing_check_test.sh' libs/starlark/tests/negative/negative_tests.bzl &&
  grep -q -F -e 'missing_observation_test.sh' libs/starlark/tests/negative/negative_tests.bzl &&
  grep -q -F -e 'missing_fragment_test.sh' libs/starlark/tests/negative/negative_tests.bzl &&
  grep -q -F -e 'fixture_real_markdown_no_config_test' quality/testdata/BUILD.bazel; then
  ok
else
  bad "green hermetic negative proofs missing (failure_test + 3 sh harnesses + markdown-no-config test)"
fi

# B4: CI test job runs the green proofs via `bazel test //...` (no separate
# manual_negatives prove step per #406).
if grep -q -F -e 'bazel test --noshow_progress //...' .github/workflows/ci.yml &&
  ! grep -q -F -e 'bazel run --noshow_progress //tools/ci:manual_negatives' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml must run green proofs via test //... with no manual_negatives step (issue #406)"
fi

# B5: starlark docs own the green-proof record plus the CI pin.
if grep -q -F -e 'failure_test' docs/testing/starlark.md &&
  grep -q -F -e 'bazel test //libs/starlark/tests/negative' docs/testing/starlark.md; then
  ok
else
  bad "docs/testing/starlark.md lost the green-proof plus CI-pin record (issue #406)"
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

# D4: every checked-in .sh rides deps(//...) (issue #407: nested E2E
# deleted, so the two integration POSIX fixtures are gone; zero unowned).
git ls-files '*.sh' | LC_ALL=C sort -u >"$scratch/all_sh.txt"
bazel query "kind('source file', deps(//...))" 2>/dev/null |
  grep -E '^(@@)?//' |
  sed 's/^@@//; s|^//||; s|:|/|; s|^/||' |
  grep '\.sh$' |
  grep -v '^@' |
  LC_ALL=C sort -u >"$scratch/owned_sh_raw.txt"
comm -23 "$scratch/all_sh.txt" "$scratch/owned_sh_raw.txt" >"$scratch/unowned_sh.txt" || true
unowned_count="$(wc -l <"$scratch/unowned_sh.txt" | tr -d ' ')"
if [[ "$unowned_count" == "0" ]]; then
  ok
else
  bad "shell ownership broke (want 0 unowned .sh, found $unowned_count: $(tr '\n' ' ' <"$scratch/unowned_sh.txt"))"
fi

# D5: shell execution paths stay wired (test job for sh tests, no nested suite).
if grep -q -F -e 'bazel test --noshow_progress //...' .github/workflows/ci.yml &&
  ! grep -q -F -e 'bazel test --noshow_progress //tools/ci:e2e' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_contract' .github/workflows/ci.yml; then
  ok
else
  bad "CI lost a shell execution path (want test //... + shell_contract, no :e2e)"
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
