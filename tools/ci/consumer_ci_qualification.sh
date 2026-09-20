#!/usr/bin/env bash
# Consumer-CI qualification harness (issue #509).
#
# Qualifies the as-built consumer-CI record with fixture evidence and
# owned gaps, without claiming unqualified integration:
# - delivered: reusable workflow contract (nine checks, explicit platforms,
#   platforms-gate fail-closed, stable dx-ci aggregate, hygiene, concurrency,
#   permissions, per-cell coverage with fork-safe comments), caller template
#   with reviewed SHA pin, four consumer fixture harnesses
#   (scheduling/aggregate/guards/pins), self-call all-enabled dogfood in
#   ci.yml (issue #408, verbatim `//...`),
#   native widen-one loop (sole updater, issue #461), dx migrate
#   planning plus
#   dx run multirun (issue #463 delivered), tag hygiene as-built;
# - open under #509 with honest records: platform/runner/isolation/cache/
#   ordering evidence; merge/diff/queue/cancellation/aggregate binding;
#   thread identity/ordering/limits; fork/untrusted/sensitive/retries/
#   Code-Scanning qualification; sequential mode fail-closed pending
#   qualification; tag hygiene plus
#   release-input gaps; native-bot follow-ups; dx migrate syntax gap
#   (run multirun delivered under #463).
#
# Versioned here, run by CI via `bazel run //tools/ci:consumer_ci_qualification`,
# following //tools/ci:docs_pipeline_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

workflow=".github/workflows/reusable-consumer.yml"
caller="examples/consumer-ci/caller.yml"
module="MODULE.bazel"
ci=".github/workflows/ci.yml"
contract="docs/github-ci.md"
matrix="docs/testing/github-ci.md"
verify="docs/testing/verification-matrix.md"
automation="docs/contributing/automation.md"
roadmap="docs/roadmap.md"
bump_workflow=".github/workflows/bump.yml"
migrate_rs="cli/adopt/src/migrate.rs"
migrate_doc="docs/cli/commands/migrate.md"
run_rs="cli/cli/src/exec/run.rs"
run_doc="docs/cli/commands/build-test-coverage.md"

# Reusable workflow declares the nine-check contract with gate plus aggregate.
if [[ -f "$workflow" ]] &&
  grep -q -F -e 'Nine checks' "$workflow" &&
  grep -q -F -e 'platforms-gate' "$workflow" &&
  grep -q -F -e 'dx-ci' "$workflow" &&
  grep -q -F -e 'Aggregate `dx-ci`' "$workflow"; then
  ok
else
  bad "reusable-consumer lost its nine-check plus gate plus aggregate contract"
fi

# Platforms-gate pins the five supported platforms with explicit selection.
if grep -q -F -e 'supported = {"linux_x86_64", "linux_arm64", "macos_arm64", "macos_x86_64", "windows_x86_64"}' "$workflow" &&
  grep -q -F -e 'no implicit default' "$workflow" &&
  grep -q -F -e 'must be a nonempty JSON array' "$workflow"; then
  ok
else
  bad "platforms-gate lost its explicit plus supported-platform pin"
fi

# Per-platform jobs route each platform to its runner: seed Linux x86_64
# to ubuntu-latest, Linux arm64 native (issue #410) to ubuntu-24.04-arm,
# macOS arm64 (issue #412) to macos-14, macOS x86_64 best-effort (issue
# #413) to macos-15-intel, Windows x86_64 MSVC-compatible (issue #414) to
# windows-latest. Linux arm64 must never fall through to macOS; macOS
# x86_64 must never fall through to the arm64 runner; Windows must never
# fall through to macOS.
if [[ "$(grep -c -F -e "matrix.platform == 'linux_arm64' && 'ubuntu-24.04-arm'" "$workflow")" == "3" ]] &&
  grep -q -F -e "macos_x86_64' && 'macos-15-intel'" "$workflow" &&
  grep -q -F -e "macos_arm64' && 'macos-14'" "$workflow" &&
  grep -q -F -e 'windows_x86_64' "$workflow" &&
  grep -q -F -e 'windows-latest' "$workflow"; then
  ok
else
  bad "per-platform jobs lost the linux_arm64/macos/windows runner mappings (arm64 to ubuntu-24.04-arm, macos_arm64 to macos-14, macos_x86_64 to macos-15-intel, windows_x86_64 to windows-latest)"
fi

# Sequential stays rejected fail-closed plus unknown-mode rejection.
if grep -q -F -e "scheduling_mode 'sequential' is qualification-open" "$workflow" &&
  grep -q -F -e "unsupported scheduling_mode" "$workflow" &&
  grep -q -F -e 'Sequential ordering is [qualification-open]' "$contract"; then
  ok
else
  bad "sequential fail-closed plus unknown-mode rejection lost"
fi

# Scheduling isolation as-built: per-platform jobs wait on the gate,
# Linux-once jobs never do.
if grep -A3 -e '^  test:' "$workflow" | grep -q -F -e 'needs: [platforms-gate]' &&
  grep -A3 -e '^  build:' "$workflow" | grep -q -F -e 'needs: [platforms-gate]' &&
  grep -A3 -e '^  coverage:' "$workflow" | grep -q -F -e 'needs: [platforms-gate]' &&
  ! grep -A3 -e '^  lint:' "$workflow" | grep -q -F -e 'needs:'; then
  ok
else
  bad "scheduling isolation lost (per-platform gate edge or Linux-once independence)"
fi

# Nine disabled_checks guards stay present, one per check ID.
guards_ok=1
for id in lint typecheck format generate security-audit license-audit test build coverage; do
  if ! grep -q -F -e "inputs.disabled_checks), ',$id,') }}" "$workflow"; then
    guards_ok=0
  fi
done
if [[ "$guards_ok" == "1" ]]; then
  ok
else
  bad "reusable-consumer lost a disabled_checks guard"
fi

# Stable aggregate: needs gate plus nine, always(), disabled-as-skipped green.
if grep -q -F -e 'needs: [platforms-gate, lint, typecheck, format, generate, security-audit, license-audit, test, build, coverage]' "$workflow" &&
  grep -A3 -e '^  dx-ci:' "$workflow" | grep -q -F -e 'if: ${{ always() }}' &&
  grep -q -F -e 'failing checks' "$workflow" &&
  grep -q -F -e 'Disabled checks report skipped and stay green' "$workflow"; then
  ok
else
  bad "dx-ci lost its stable needs plus always plus skipped-green record"
fi

# Four consumer fixture harnesses stay wired as evidence.
if [[ -f "tools/ci/consumer_scheduling_test.sh" ]] &&
  [[ -f "tools/ci/consumer_aggregate_test.sh" ]] &&
  [[ -f "tools/ci/consumer_guards_test.sh" ]] &&
  [[ -f "tools/ci/consumer_pins_test.sh" ]] &&
  grep -q -F -e 'consumer_scheduling_test' tools/ci/BUILD.bazel &&
  grep -q -F -e 'consumer_aggregate_test' tools/ci/BUILD.bazel; then
  ok
else
  bad "consumer fixture harnesses missing (scheduling/aggregate/guards/pins)"
fi

# Caller template: full-SHA pin, explicit platforms, parallel, version match.
caller_version="$(grep -o -E -e 'rules_dx_version: "[^"]+"' "$caller" | head -n 1 | cut -d'"' -f2)"
module_version="$(awk '/^module\(/,/^\)/' "$module" | grep -o -E -e 'version = "[^"]+"' | head -n 1 | cut -d'"' -f2)"
if [[ "$(grep -c -E -e 'uses: rules_dx/\.github/workflows/reusable-consumer\.yml@[0-9a-f]{40}' "$caller")" == "1" ]] &&
  grep -q -F -e "platforms: '[\"linux_x86_64\"]'" "$caller" &&
  grep -q -F -e 'scheduling_mode: "parallel"' "$caller" &&
  [[ -n "$caller_version" && "$caller_version" == "$module_version" ]]; then
  ok
else
  bad "caller template lost SHA pin, platforms, parallel, or version match ($caller_version vs $module_version)"
fi

# Hygiene: SHA pins with tag comments, single setup-bazelisk, no inline install.
actions_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40}' "$workflow" "$caller" | wc -l)"
commented_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40} # v[0-9]+' "$workflow" "$caller" | wc -l)"
if [[ "$actions_pins" -gt "0" && "$actions_pins" == "$commented_pins" ]] &&
  [[ "$(grep -c -F -e './.github/actions/setup-bazelisk' "$workflow")" -ge "9" ]] &&
  ! grep -e 'setup-bazelisk' "$workflow" | grep -v -F -e './.github/actions/setup-bazelisk' | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  ok
else
  bad "hygiene lost (SHA plus tag comments or single setup-bazelisk path)"
fi

# Concurrency cancels superseded PR runs without deleting history.
if grep -q -F -e 'group: dx-ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}' "$workflow" &&
  grep -q -F -e 'cancel-in-progress: true' "$workflow" &&
  grep -q -F -e 'New PR commits automatically cancel superseded' "$contract"; then
  ok
else
  bad "concurrency supersession record lost (group plus cancel-in-progress)"
fi

# Permissions stay least-privilege with fork-safe checkout.
if grep -q -F -e 'contents: read' "$workflow" &&
  grep -q -F -e 'checks: write' "$workflow" &&
  grep -q -F -e 'pull-requests: write' "$workflow" &&
  [[ "$(grep -c -F -e 'persist-credentials: false' "$workflow")" -ge "9" ]]; then
  ok
else
  bad "permissions lost least-privilege plus persist-credentials record"
fi

# Coverage stays per-cell no-union with Codecov opt-in only.
if grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' "$workflow" &&
  grep -q -F -e 'no cross-cell union' "$workflow" &&
  grep -q -F -e 'Codecov stays opt-in only' "$workflow"; then
  ok
else
  bad "consumer coverage lost per-cell no-union plus Codecov opt-in record"
fi

# Fork-safe PR comment: dedup marker, fork skip, no write creds for fork.
if grep -q -F -e 'dx-coverage-summary: coverage' "$workflow" &&
  grep -q -F -e 'Fork PRs skip so fork code never gets write' "$workflow" &&
  grep -q -F -e 'Fork PR detected' "$workflow" &&
  grep -q -F -e 'Privileged reporting stays separate and never executes fork-controlled code' "$contract"; then
  ok
else
  bad "fork-safe comment wiring lost (marker plus skip plus no-creds)"
fi

# Self-call in ci.yml runs all nine checks (issue #408 dogfood-like
# consumer) on all qualified hosts (seed plus arm64 native plus macOS
# arm64 plus macOS x86_64 best-effort plus Windows x86_64, issues
# #410/#412/#413/#414). No bespoke corpus scope remains in ci.yml: dogfood
# is verbatim `//...` via the reusable workflow.
if grep -q -F -e 'consumer-ci (self-call reusable consumer workflow)' "$ci" &&
  grep -q -F -e 'disabled_checks: ""' "$ci" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"macos_x86_64\", \"windows_x86_64\"]'" "$ci" &&
  ! grep -q -F -e 'attr(tags, corpus' "$ci" &&
  ! grep -q -F -e 'Only build is enabled' "$ci"; then
  ok
else
  bad "ci.yml self-call lost all-enabled plus explicit-platforms plus no-corpus honesty"
fi

# Functional gate: sequential fails closed, parallel linux passes.
dx_mkscratch scratch
command -v python3 >/dev/null || {
  echo "python3 is required" >&2
  exit 1
}
awk "/python3 - <<'EOF'/{count++; f=(count==1); next} f && /^[[:space:]]*EOF$/{exit} f{sub(/^          /, \"\"); print}" "$workflow" >"$scratch/gate.py"
gate_seq_out="$(SCHEDULING="sequential" PLATFORMS='["linux_x86_64"]' DISABLED="" python3 "$scratch/gate.py" 2>&1 || true)"
gate_par_out="$(SCHEDULING="parallel" PLATFORMS='["linux_x86_64"]' DISABLED="" python3 "$scratch/gate.py" 2>&1 || true)"
gate_empty_out="$(SCHEDULING="parallel" PLATFORMS="" DISABLED="" python3 "$scratch/gate.py" 2>&1 || true)"
if grep -q -F -e 'no per-platform check enabled' "$scratch/gate.py" &&
  echo "$gate_seq_out" | grep -q -F -e 'qualification-open' &&
  echo "$gate_par_out" | grep -q -F -e 'platforms-gate:' &&
  echo "$gate_empty_out" | grep -q -F -e 'no implicit default'; then
  ok
else
  bad "functional gate proof failed (sequential/parallel/empty)"
fi

# Functional aggregate: all-success green, single failure names check,
# disabled-as-skipped green.
awk "/python3 - <<'EOF'/{count++; f=(count==2); next} f && /^[[:space:]]*EOF$/{exit} f{sub(/^          /, \"\"); print}" "$workflow" >"$scratch/aggregate.py"
all_ok='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"success"},"build":{"result":"success"},"coverage":{"result":"success"}}'
one_fail='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"failure"},"build":{"result":"success"},"coverage":{"result":"success"}}'
disabled_skip='{"platforms-gate":{"result":"success"},"lint":{"result":"skipped"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"success"},"build":{"result":"success"},"coverage":{"result":"success"}}'
agg_ok_rc=0
NEEDS="$all_ok" python3 "$scratch/aggregate.py" >/dev/null 2>&1 || agg_ok_rc=$?
agg_fail_out="$(NEEDS="$one_fail" python3 "$scratch/aggregate.py" 2>&1 || true)"
agg_skip_rc=0
NEEDS="$disabled_skip" python3 "$scratch/aggregate.py" >/dev/null 2>&1 || agg_skip_rc=$?
if [[ "$agg_ok_rc" == "0" ]] &&
  echo "$agg_fail_out" | grep -q -F -e 'failing checks: test' &&
  [[ "$agg_skip_rc" == "0" ]]; then
  ok
else
  bad "functional aggregate proof failed (all-ok/one-fail/disabled-skip)"
fi

# Native widen-one loop stays delivered as the sole updater (issue #461).
if grep -q -F -e 'BumpSet::Bazel' cli/bump/src/sets.rs &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'native-only' "$automation" &&
  [[ -f "$bump_workflow" ]] &&
  grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q . &&
  grep -q -F -e 'dx bump' "$automation" &&
  [[ -f "tools/ci/widen_update_loop.sh" ]]; then
  ok
else
  bad "native bump loop lost (set registry, sole updater, bump command, runner)"
fi

# dx migrate planning delivered with fail-closed execution honesty.
if grep -q -F -e 'pub fn migrate_is_major_bump' "$migrate_rs" &&
  grep -q -F -e 'pub fn migrate_manifest_name' "$migrate_rs" &&
  grep -q -F -e 'pub fn plan_migrate' "$migrate_rs" &&
  grep -q -F -e 'migrate_is_major_release_only' "$migrate_rs" &&
  grep -q -F -e 'planning library delivered' "$migrate_doc" &&
  grep -q -F -e 'No manifests exist yet' "$migrate_doc"; then
  ok
else
  bad "dx migrate planning lost (major-bump plus manifest plus fail-closed doc)"
fi

# dx run multirun delivered (issue #463): sequential, local-only, single plus multi.
if grep -q -F -e 'execute_run_multi' "$run_rs" &&
  grep -q -F -e 'multirun #186' "$run_rs" &&
  grep -q -F -e 'dx run refuses when CI=true: local-only command' "$run_rs" &&
  grep -q -F -e 'Multiple explicit labels run sequentially' "$run_doc" &&
  grep -q -F -e 'Strict single-target execution applies to file/directory' "$run_doc" &&
  grep -q -F -e 'issue #463' "$run_doc"; then
  ok
else
  bad "dx run multirun lost (multi plus sequential plus local-only plus docs plus #463)"
fi

# Tag hygiene as-built: unpublishable module, no tags, ignored outputs.
if grep -q -F -e 'version = "0.0.0"' "$module" &&
  [[ -z "$(git tag --list 'v*' || true)" ]] &&
  grep -q -F -e '/dist/' .gitignore &&
  grep -q -F -e '/release/' .gitignore &&
  [[ -f "tools/ci/release_hygiene.sh" ]] &&
  [[ -f "tools/ci/publish_trust.sh" ]]; then
  ok
else
  bad "tag hygiene as-built lost (0.0.0 plus no tags plus ignored outputs plus harnesses)"
fi

# Platform/runner/isolation/cache/ordering evidence stays owned open.
if grep -q -F -e 'Map supported platform identifiers, runner/OS/architecture identities' "$contract" &&
  grep -q -F -e 'scheduling isolation, cache/resource use, sequential ordering' "$contract" &&
  grep -q -F -e 'platform, runner, isolation, cache, ordering evidence' "$matrix"; then
  ok
else
  bad "platform/runner/isolation/cache/ordering gap lost its owner"
fi

# Merge/diff/queue/cancellation/aggregate binding stays owned open.
if grep -q -F -e 'merge snapshots and diff mapping' "$contract" &&
  grep -q -F -e 'queue lifecycle, base advancement, cancellation races' "$contract" &&
  grep -q -F -e 'aggregate required-check bindings' "$contract" &&
  grep -q -F -e 'merge, diff, queue, cancellation, aggregate binding' "$matrix"; then
  ok
else
  bad "merge/diff/queue/cancellation/aggregate gap lost its owner"
fi

# Thread identity/ordering/limits stays owned open with unfrozen limit.
if grep -q -F -e 'Freeze finding/thread identity, deterministic ordering' "$contract" &&
  grep -q -F -e 'numeric limit and accounting' "$contract" &&
  grep -q -F -e 'thread identity, ordering, limits' "$matrix" &&
  grep -q -F -e 'The numeric' "$contract" &&
  grep -q -F -e 'not yet frozen' "$contract"; then
  ok
else
  bad "thread identity/ordering/limits gap lost its owner"
fi

# Fork/untrusted/sensitive/retries/Code-Scanning stays owned open.
if grep -q -F -e 'Qualify fork roles/settings, untrusted artifact' "$contract" &&
  grep -q -F -e 'sensitive-content handling, bounded transport retries' "$contract" &&
  grep -q -F -e 'opt-in Code Scanning publication' "$contract" &&
  grep -q -F -e 'fork, untrusted, sensitive, retries, Code-Scanning qualification' "$matrix" &&
  grep -q -F -e 'code_scanning_opt_in' "$workflow" &&
  ! grep -rn -F -e 'upload-sarif' .github/workflows/reusable-consumer.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "fork/untrusted/sensitive/retries/Code-Scanning gap lost its owner"
fi

# Contract plus matrix keep the #509 qualification tracker.
if grep -q -F -e 'issue #509' "$contract" &&
  grep -q -F -e 'issue #509' "$matrix"; then
  ok
else
  bad "github-ci contract/matrix lost its #509 qualification tracker record"
fi

# Tag hygiene plus release-input gaps stay owned open.
if grep -q -F -e 'Tag hygiene and release-input gaps' "$roadmap" &&
  grep -q -F -e 'tag hygiene plus release-input gaps' "$matrix"; then
  ok
else
  bad "tag hygiene plus release-input gap lost its owner"
fi

# Native-bot follow-ups stay owned open (native-only updater, issue #461).
if grep -q -F -e 'native-bot follow-ups' "$matrix" &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'sole updater' "$roadmap"; then
  ok
else
  bad "native-bot follow-up gap lost its owner"
fi

# dx migrate syntax stays owned open in the matrix; dx run multirun is
# delivered under #463 and must not regress to the combined open record.
if grep -q -F -e 'dx migrate syntax)' "$matrix" &&
  ! grep -q -F -e 'dx migrate syntax plus dx run multirun' "$matrix" &&
  grep -q -F -e 'V1 scope: `dx migrate` syntax + manifest selection (issue #462).' "$roadmap" &&
  grep -q -F -e 'delivered (issue #463' "$roadmap"; then
  ok
else
  bad "dx migrate gap lost its owner or dx run multirun regressed to open"
fi

# Verification matrix keeps no Supported claim with consumer honesty.
if ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$verify" | grep -q . &&
  grep -q -F -e 'self-call all-enabled' "$matrix"; then
  ok
else
  bad "verification matrix lost its no-Supported plus consumer-honesty gate"
fi

dx_test_summary "consumer CI qualification harness"
