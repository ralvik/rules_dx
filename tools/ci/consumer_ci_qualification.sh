#!/usr/bin/env bash
# Consumer-CI qualification harness.
#
# Qualifies the as-built consumer-CI record with fixture evidence pinned in
# `tools/ci/tests/fixtures/consumer_ci/pins.bzl` (plus `platforms.expected`
# plus `revisions.expected` plus `reporting.expected`), without claiming
# unqualified integration:
# - delivered: reusable workflow contract (nine checks, explicit platforms,
#   platforms-gate fail-closed, stable dx-ci aggregate, hygiene, concurrency,
#   permissions, per-cell coverage with fork-safe comments), caller template
#   with reviewed SHA pin, four consumer fixture harnesses
#   (scheduling/aggregate/guards/pins), self-call test-disabled dogfood in
# ci.yml
#   disabled as coverage superset via `resolve_for_test` plus `bazel coverage`),
# native widen-one loop (sole updater,), dx migrate syntax
#   plus manifest selection (delivered CLI with fail-closed execution, issue
# plus dx run multirun (delivered), tag hygiene as-built;
# - per-gap decisions with fixtures: platform/runner/isolation/cache/
#   ordering evidence; merge/diff/queue/cancellation/aggregate binding;
#   thread identity/ordering/limits; fork/untrusted/sensitive/retries/
#   Code-Scanning qualification; sequential mode fail-closed pending
#   qualification; tag hygiene plus
#   release-input gaps; native-bot follow-ups; dx migrate manifest gap
# (syntax plus selection delivered under, run multirun delivered
# under); build-only self-call forever rejected per.
# Seed only for platform plus consumer plus release evidence; no
# Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:consumer_ci_qualification`,
# following //tools/ci:docs_pipeline_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

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
pins="tools/ci/tests/fixtures/consumer_ci/pins.bzl"
pins_build="tools/ci/tests/fixtures/consumer_ci/BUILD.bazel"
platforms_expected="tools/ci/tests/fixtures/consumer_ci/platforms.expected"
revisions_expected="tools/ci/tests/fixtures/consumer_ci/revisions.expected"
reporting_expected="tools/ci/tests/fixtures/consumer_ci/reporting.expected"

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
# to ubuntu-latest, Linux arm64 native to ubuntu-24.04-arm,
# macOS arm64 to macos-14, macOS x86_64 best-effort (issue
# to macos-15-intel, Windows x86_64 MSVC-compatible to
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

# Self-call in ci.yml runs eight checks with `test` disabled
# dogfood-like consumer plus Phase 1 coverage superset) on all
# qualified hosts (seed plus arm64 native plus macOS arm64 plus macOS x86_64
# best-effort plus Windows x86_64,). `dx coverage`
# resolves scope via `resolve_for_test` (same as `dx test`) and runs `bazel
# coverage`, which executes the tests. No bespoke corpus scope remains in
# ci.yml: dogfood is verbatim `//...` via the reusable workflow.
if grep -q -F -e 'dogfood (self-call reusable consumer workflow)' "$ci" &&
  grep -q -F -e 'disabled_checks: "test"' "$ci" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"macos_x86_64\", \"windows_x86_64\"]'" "$ci" &&
  ! grep -q -F -e 'attr(tags, corpus' "$ci" &&
  ! grep -q -F -e 'Only build is enabled' "$ci"; then
  ok
else
  bad "ci.yml self-call lost test-disabled plus explicit-platforms plus no-corpus honesty (issue #607 coverage superset)"
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

# Native widen-one loop stays delivered as the sole updater.
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

# dx migrate syntax plus manifest selection delivered with fail-closed execution honesty.
if grep -q -F -e 'pub fn migrate_is_major_bump' "$migrate_rs" &&
  grep -q -F -e 'pub fn migrate_manifest_name' "$migrate_rs" &&
  grep -q -F -e 'pub fn plan_migrate' "$migrate_rs" &&
  grep -q -F -e 'migrate_is_major_release_only' "$migrate_rs" &&
  grep -q -F -e 'migrate_manifest_selection_is_mechanical' "$migrate_rs" &&
  grep -q -F -e 'Migrate,' cli/cli/src/args/command.rs &&
  grep -q -F -e 'execute_migrate' cli/cli/src/exec/migrate.rs &&
  grep -q -F -e 'migrate_failed' cli/cli/src/exec/migrate.rs &&
  grep -q -F -e 'delivered CLI' "$migrate_doc" &&
  grep -q -F -e 'No manifests exist yet' "$migrate_doc" &&
  grep -q -F -e 'migrate-v<from_major>-to-v<to_major>.json' "$migrate_doc"; then
  ok
else
  bad "dx migrate syntax plus manifest selection lost (major-bump plus manifest plus CLI plus fail-closed doc)"
fi

# dx run multirun delivered: sequential, local-only, single plus multi.
if grep -q -F -e 'execute_run_multi' "$run_rs" &&
  grep -q -F -e 'explicit-label multirun' "$run_rs" &&
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

# Thread identity/ordering stays owned open with limit plus accounting frozen under.
if grep -q -F -e 'Freeze finding/thread identity, deterministic ordering' "$contract" &&
  grep -q -F -e 'numeric limit plus thread-accounting frozen at 50 open threads under issue #592' "$contract" &&
  grep -q -F -e 'thread identity, ordering, limits' "$matrix" &&
  grep -q -F -e 'frozen at 50 open integration-owned threads' "$contract" &&
  grep -q -F -e 'issue #592' "$contract"; then
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

# Contract plus matrix keep the qualification tracker.
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

# Native-bot follow-ups stay owned open (native-only updater,).
if grep -q -F -e 'native-bot follow-ups' "$matrix" &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'sole updater' "$roadmap"; then
  ok
else
  bad "native-bot follow-up gap lost its owner"
fi

# dx migrate syntax plus manifest selection plus dx run multirun stay
# delivered in the matrix and roadmap; neither may
# regress to the combined open record.
if grep -q -F -e 'migrate syntax plus' "$matrix" &&
  grep -q -F -e 'issue #462' "$matrix" &&
  grep -q -F -e 'issue #463' "$matrix" &&
  grep -q -F -e 'V1 scope: `dx migrate` syntax + manifest selection delivered' "$roadmap" &&
  grep -q -F -e 'delivered (issue #463' "$roadmap"; then
  ok
else
  bad "dx migrate plus dx run gap lost its delivered owner"
fi

# Verification matrix keeps no Supported claim with consumer honesty.
# The dogfood self-call stays test-disabled per Phase 1 
# (coverage superset); the starter caller stays all-nine.
if ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$verify" | grep -q . &&
  grep -q -F -e 'self-call test-disabled' "$matrix"; then
  ok
else
  bad "verification matrix lost its no-Supported plus consumer-honesty gate"
fi

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$platforms_expected" && -f "$revisions_expected" && -f "$reporting_expected" ]]; then
  ok
else
  bad "consumer-CI fixture missing (want $pins plus $pins_build plus platforms.expected plus revisions.expected plus reporting.expected)"
fi

# Pins record platform plus runner decisions with no implicit default.
if grep -q -F -e 'PLATFORM_IDS' "$pins" &&
  grep -q -F -e 'supported = {"linux_x86_64", "linux_arm64", "macos_arm64", "macos_x86_64", "windows_x86_64"}' "$pins" &&
  grep -q -F -e 'no implicit default' "$pins" &&
  grep -q -F -e 'must be a nonempty JSON array' "$pins" &&
  grep -q -F -e "matrix.platform == 'linux_arm64' && 'ubuntu-24.04-arm'" "$pins" &&
  grep -q -F -e "macos_x86_64' && 'macos-15-intel'" "$pins" &&
  grep -q -F -e "macos_arm64' && 'macos-14'" "$pins" &&
  grep -q -F -e 'windows-latest' "$pins" &&
  grep -q -F -e 'never fall through' "$pins" &&
  grep -q -F -e 'shell: bash' "$pins"; then
  ok
else
  bad "pins.bzl lost its platform plus runner pins under issue #509"
fi

# Pins record isolation plus cache plus ordering decisions.
if grep -q -F -e 'persist-credentials: false' "$pins" &&
  grep -q -F -e 'needs: [platforms-gate]' "$pins" &&
  grep -q -F -e 'Linux-once jobs never do' "$pins" &&
  grep -q -F -e 'do not advertise parallelism while jobs serialize on one shared Bazel output-base lock' "$pins" &&
  grep -q -F -e 'fail-fast: false' "$pins" &&
  grep -q -F -e 'no actions/cache in reusable-consumer' "$pins" &&
  grep -q -F -e 'bazel-windows-x86_64-' "$pins" &&
  grep -q -F -e 'free-tier eligible' "$pins" &&
  grep -q -F -e 'scheduling_mode: "parallel"' "$pins" &&
  grep -q -F -e "scheduling_mode 'sequential' is qualification-open" "$pins" &&
  grep -q -F -e 'unsupported scheduling_mode' "$pins"; then
  ok
else
  bad "pins.bzl lost its isolation plus cache plus ordering pins under issue #509"
fi

# Pins record merge plus diff plus queue decisions.
if grep -q -F -e 'Validate the proposed merge, not the contributor branch alone' "$pins" &&
  grep -q -F -e 'All selected checks in a run use the same test-merge snapshot' "$pins" &&
  grep -q -F -e 'report validation as blocked without head-only fallback or aggregate success' "$pins" &&
  grep -q -F -e 'diff-based review placement does not narrow analysis scope' "$pins" &&
  grep -q -F -e 'never on invented lines' "$pins" &&
  grep -q -F -e 'Consumers enable and configure their queue' "$pins" &&
  grep -q -F -e 'bound to the queue revision rather than an individual PR head' "$pins" &&
  grep -q -F -e 'Queue runs never create, update, or clean up PR comments/threads' "$pins" &&
  grep -q -F -e 'Queue membership does not grant fork code secrets or write credentials' "$pins"; then
  ok
else
  bad "pins.bzl lost its merge plus diff plus queue pins under issue #509"
fi

# Pins record cancellation plus aggregate decisions.
if grep -q -F -e 'group: dx-ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}' "$pins" &&
  grep -q -F -e 'cancel-in-progress: true' "$pins" &&
  grep -q -F -e "Scope PR supersession to that PR's integration runs" "$pins" &&
  grep -q -F -e 'Late callbacks must not overwrite current summaries' "$pins" &&
  grep -q -F -e 'needs: [platforms-gate, lint, typecheck, format, generate, security-audit, license-audit, test, build, coverage]' "$pins" &&
  grep -q -F -e 'if: ${{ always() }}' "$pins" &&
  grep -q -F -e 'Disabled checks report skipped and stay green' "$pins" &&
  grep -q -F -e 'cannot produce success' "$pins" &&
  grep -q -F -e 'require in branch protection' "$pins"; then
  ok
else
  bad "pins.bzl lost its cancellation plus aggregate pins under issue #509"
fi

# Pins record thread plus limit decisions with frozen numeric under.
if grep -q -F -e 'replyable/resolvable PR review threads' "$pins" &&
  grep -q -F -e 'There is no review-comment opt-out or annotation-only mode' "$pins" &&
  grep -q -F -e 'Preserve human comments, replies, and unrelated threads' "$pins" &&
  grep -q -F -e 'delete bot-only threads without replies' "$pins" &&
  grep -q -F -e 'resolve threads with human replies instead' "$pins" &&
  grep -q -F -e 'one fixed, documented per-PR review-thread limit' "$pins" &&
  grep -q -F -e 'without a consumer setting' "$pins" &&
  grep -q -F -e 'deterministic ordering, not job completion order' "$pins" &&
  grep -q -F -e 'Full reports retain every finding' "$pins" &&
  grep -q -F -e 'frozen at 50 open integration-owned threads under issue #592' "$pins"; then
  ok
else
  bad "pins.bzl lost its thread plus limit pins under issue #509"
fi

# Pins record fork plus untrusted plus sensitive decisions.
if grep -q -F -e 'all_external_contributors' "$pins" &&
  grep -q -F -e 'outside authors cannot self-approve' "$pins" &&
  grep -q -F -e 'does not give fork code secrets or write credentials' "$pins" &&
  grep -q -F -e 'dx-coverage-summary: coverage' "$pins" &&
  grep -q -F -e 'Fork PR detected' "$pins" &&
  grep -q -F -e 'Privileged reporting stays separate and never executes fork-controlled code' "$pins" &&
  grep -q -F -e 'Treat artifacts and PR metadata as untrusted' "$pins" &&
  grep -q -F -e 'contents: read' "$pins" &&
  grep -q -F -e 'does not authorize exposing credentials, secret values' "$pins" &&
  grep -q -F -e 'public CI reporting is not confidential' "$pins"; then
  ok
else
  bad "pins.bzl lost its fork plus untrusted plus sensitive pins under issue #509"
fi

# Pins record retries plus Code-Scanning plus sequential decisions.
if grep -q -F -e "Use GitHub's native rerun controls, not bot comment commands" "$pins" &&
  grep -q -F -e 'Bounded transient reporting-transport retries may reuse the same identified results' "$pins" &&
  grep -q -F -e 'exhausted retries retain reporting failure' "$pins" &&
  grep -q -F -e 'code_scanning_opt_in' "$pins" &&
  grep -q -F -e 'Off by default' "$pins" &&
  grep -q -F -e 'upload-sarif' "$pins" &&
  grep -q -F -e 'Disabled publication is not a reporting failure' "$pins" &&
  grep -q -F -e 'Sequential ordering is [qualification-open]' "$pins" &&
  grep -q -F -e 'fails closed on `scheduling_mode: sequential`' "$pins"; then
  ok
else
  bad "pins.bzl lost its retries plus Code-Scanning plus sequential pins under issue #509"
fi

# Pins record tag plus release plus native-bot plus rejected decisions.
if grep -q -F -e 'version = "0.0.0"' "$pins" &&
  grep -q -F -e 'no v* tags' "$pins" &&
  grep -q -F -e '/dist/' "$pins" &&
  grep -q -F -e 'no silent upgrades' "$pins" &&
  grep -q -F -e 'Must match the consumer MODULE.bazel pin' "$pins" &&
  grep -q -F -e 'Tag hygiene and release-input gaps' "$pins" &&
  grep -q -F -e 'sole updater' "$pins" &&
  grep -q -F -e 'native-only' "$pins" &&
  grep -q -F -e '"build-only self-call forever"' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its tag plus release plus native-bot plus rejected pins under issue #509"
fi

# Fixture expected texts cover platform plus revision plus reporting gaps.
if grep -q -F -e 'Five explicit platform identifiers with no implicit default' "$platforms_expected" &&
  grep -q -F -e 'never falls through' "$platforms_expected" &&
  grep -q -F -e 'absent in the reusable workflow' "$platforms_expected" &&
  grep -q -F -e 'parallel only' "$platforms_expected" &&
  grep -q -F -e 'Validate the proposed merge' "$revisions_expected" &&
  grep -q -F -e 'never narrows scope' "$revisions_expected" &&
  grep -q -F -e 'consumer-native' "$revisions_expected" &&
  grep -q -F -e 'concurrency-scoped' "$revisions_expected" &&
  grep -q -F -e 'single stable dx-ci' "$revisions_expected" &&
  grep -q -F -e 'replyable/resolvable' "$reporting_expected" &&
  grep -q -F -e 'one fixed, documented per-PR' "$reporting_expected" &&
  grep -q -F -e 'frozen at 50 open integration-owned' "$reporting_expected" &&
  grep -q -F -e 'all_external_contributors' "$reporting_expected" &&
  grep -q -F -e 'least-privilege' "$reporting_expected" &&
  grep -q -F -e 'native rerun' "$reporting_expected" &&
  grep -q -F -e 'opt-in via code_scanning_opt_in' "$reporting_expected" &&
  grep -q -F -e 'qualification-open' "$reporting_expected" &&
  grep -q -F -e 'version 0.0.0' "$reporting_expected"; then
  ok
else
  bad "platforms/revisions/reporting.expected lost gap coverage (want platform plus merge plus thread plus fork plus sequential plus tag, issue #509)"
fi

# Docs own the qualified record with the fixture proof.
if grep -q -F -e 'tools/ci/tests/fixtures/consumer_ci/pins.bzl' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/consumer_ci/pins.bzl' "$matrix" &&
  grep -q -F -e 'qualified by `bazel run //tools/ci:consumer_ci_qualification`' "$matrix"; then
  ok
else
  bad "github-ci contract or matrix lost its #509 pins fixture record"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/consumer_ci/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "consumer-CI fixture failed to build (want green on the seed host, issue #509)"
fi

# Verification matrix owns the qualified record under.
if grep -q -F -e 'Consumer-CI per-gap decisions with fixture evidence' "$verify" &&
  grep -q -F -e 'qualified seed-only under #509' "$verify" &&
  grep -q -F -e 'tools/ci/tests/fixtures/consumer_ci/pins.bzl' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:consumer_ci_qualification' "$verify" &&
  grep -q -F -e '`consumer_ci_qualification` 43/43' "$verify"; then
  ok
else
  bad "verification-matrix lost its #509 per-gap qualified record with 43/43"
fi

dx_test_summary "consumer CI qualification harness"
