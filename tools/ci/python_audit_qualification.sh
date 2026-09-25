#!/usr/bin/env bash
# Python source-audit selection qualification harness.
#
# Selects Ruff S (flake8-bandit) via the pinned Ruff standalone artifact
# as the Python source-audit tool beyond the empty curated set (successor
# to closed #613), with fixture evidence:
# - selection: ruff audit over python plus python_stub via the S ruleset,
#   Ruff 0.16.7 standalone checksummed artifact with no new acquisition,
#   S via native ruff.toml opt-in with pinned upstream defaults otherwise
#   clean and no hidden preset, curated audit stays empty with explicit
#   disablement (audit opt-in, no default fetch), Bandit excluded from v1
#   by ADR 0019 (re-selection rejected here, reconsideration under #970),
#   secrets ride the separate Gitleaks family;
# - unaffected: python lint stays pydoclint plus ruff, format stays ruff,
#   typecheck stays ty, flake8 plus pylint stay baseline opt-ins;
# - adapters: ruff audit claims python plus python_stub check-only via
#   the hermetic ruff check JSON path shared with lint;
# - scope: per-language source audit distinct from ecosystem dx audit plus
#   dx update live execution delivered repo-wide;
# - rejected: leaving under taxonomy rejected with mismatched scope;
# - fixtures: `python/tests/fixtures/python_audit/` (`pins.bzl` plus
#   `python_audit.expected` plus audit samples) pins selection plus
#   rejected plus honesty;
# - platform: Ruff per-host standalone artifacts for the required hosts;
# - consumer: adopt-python with lazy audit opt-in;
# - release: promotion-checklist plus SBOM plus signing linkage.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:python_audit_qualification`,
# following //tools/ci:quality_taxonomy_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="python/tests/fixtures/python_audit/pins.bzl"
pins_build="python/tests/fixtures/python_audit/BUILD.bazel"
expected="python/tests/fixtures/python_audit/python_audit.expected"
clean_sample="python/tests/fixtures/python_audit/Sample_audit_clean.py"
dirty_sample="python/tests/fixtures/python_audit/Sample_audit_dirty.py"
adapters="quality/adapters.bzl"
curated="quality/curated_defaults.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
action_doc="docs/quality/action-model.md"
python_adr="docs/decisions/0010-python-foundation.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
promotion="docs/product/support-matrix.md"
adopt="examples/adopt-python"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" && -f "$clean_sample" && -f "$dirty_sample" ]]; then
  ok
else
  bad "python audit fixture missing (want $pins plus $pins_build plus python_audit.expected plus audit samples)"
fi

# Pins record the Ruff S selection plus empty curated plus Bandit plus secrets.
if grep -q -F -e 'ruff audit over python plus python_stub via S ruleset' "$pins" &&
  grep -q -F -e 'curated audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'python family audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'Bandit excluded from v1 by ADR 0019' "$pins" &&
  grep -q -F -e 'secrets family rides Gitleaks detect with redact plus SARIF' "$pins"; then
  ok
else
  bad "pins.bzl lost its Ruff S selection plus empty audit plus Bandit plus secrets split under issue #801"
fi

# Pins record the artifact plus ruleset plus acquisition plus upstream defaults.
if grep -q -F -e 'ruff 0.16.7 standalone artifact' "$pins" &&
  grep -q -F -e 'S flake8-bandit ruleset via native ruff.toml opt-in' "$pins" &&
  grep -q -F -e 'checksummed standalone artifact with no new acquisition' "$pins" &&
  grep -q -F -e 'pinned upstream defaults otherwise clean with no hidden preset' "$pins"; then
  ok
else
  bad "pins.bzl lost its artifact plus ruleset plus acquisition plus upstream defaults under issue #801"
fi

# Pins record the audit adapter plus unaffected quality.
if grep -q -F -e 'ruff audit claims python plus python_stub check-only via ruff check JSON' "$pins" &&
  grep -q -F -e 'python family lint pydoclint plus ruff' "$pins" &&
  grep -q -F -e 'python family format ruff' "$pins" &&
  grep -q -F -e 'python family typecheck ty' "$pins" &&
  grep -q -F -e 'flake8 plus pylint stay baseline opt-ins' "$pins"; then
  ok
else
  bad "pins.bzl lost its audit adapter plus lint plus format plus typecheck under issue #801"
fi

# Pins record scope plus rejected substitutes.
if grep -q -F -e 'per-language source audit distinct from ecosystem dx audit plus dx update live execution' "$pins" &&
  grep -q -F -e 'leaving under taxonomy rejected with mismatched scope' "$pins" &&
  grep -q -F -e 'Bandit re-selection rejected here with ADR 0019 exclusion standing' "$pins"; then
  ok
else
  bad "pins.bzl lost its scope plus taxonomy plus Bandit rejected under issue #801"
fi

# Pins record platform plus consumer plus release evidence.
if grep -q -F -e 'ruff per-host artifacts linux_x86_64 plus linux_arm64 plus macos_arm64 plus windows_x86_64' "$pins" &&
  grep -q -F -e 'standalone artifact route with per-host digests in quality/artifacts/ruff' "$pins" &&
  grep -q -F -e 'examples/adopt-python consumer with lazy audit opt-in' "$pins" &&
  grep -q -F -e 'promotion-checklist plus SBOM plus signing linkage with no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its platform plus consumer plus release evidence under issue #801"
fi

# Pins record owned selection plus taxonomy plus promotion plus honesty.
if grep -q -F -e 'future tool selection owned under issue #801 with successor to closed #613' "$pins" &&
  grep -q -F -e 'issue #512 stays taxonomy-only with no Python audit ownership' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'Supported promotion stays owned under #808 with per-host #803 through #807' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #801' "$pins"; then
  ok
else
  bad "pins.bzl lost its owned selection plus taxonomy plus promotion plus seed-only under issue #801"
fi

# Curated defaults keep python audit empty with lint plus format plus typecheck.
if grep -q -F -e '"python": {' "$curated" &&
  grep -q -F -e '"audit": []' "$curated" &&
  grep -q -F -e '"python": ["ruff"]' "$curated" &&
  grep -q -F -e 'pydoclint' "$curated" &&
  grep -q -F -e '"typecheck": ["ty"]' "$curated"; then
  ok
else
  bad "curated defaults lost python audit empty plus ruff plus pydoclint plus ty"
fi

# Ruff audit adapter claims python plus python_stub in the real registry.
if grep -q -F -e '"ruff": {"audit": ["python", "python_stub"]' "$adapters"; then
  ok
else
  bad "adapters.bzl lost its ruff audit claim over python plus python_stub under issue #801"
fi

# Tool baseline owns the #801 selection record with Bandit honesty.
if grep -q -F -e 'closed #801' "$baseline" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$baseline" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$baseline" &&
  grep -q -F -e 'Bandit excluded from v1' "$baseline" &&
  grep -q -F -e 'Ruff S' "$baseline"; then
  ok
else
  bad "docs/tools/tool-baseline.md lost its #801 audit-selection record with fixtures plus qualification"
fi

# Action model owns the audit-selection record with Ruff S plus Bandit.
if grep -q -F -e 'qualified seed-only under closed #801' "$action_doc" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$action_doc" &&
  grep -q -F -e 'python_audit.expected' "$action_doc" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$action_doc" &&
  grep -q -F -e 'Bandit excluded from v1' "$action_doc" &&
  grep -q -F -e 'Ruff S' "$action_doc"; then
  ok
else
  bad "docs/quality/action-model.md lost its #801 audit-selection record with fixtures plus qualification"
fi

# Python foundation ADR owns the source-audit selection under #801.
if grep -q -F -e 'closed #801' "$python_adr" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$python_adr" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$python_adr"; then
  ok
else
  bad "docs/decisions/0010-python-foundation.md lost its #801 source-audit selection record"
fi

# ci_targets_d.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "python_audit_qualification"' "tools/ci/ci_targets_d.bzl" ||
  grep -q -F -e 'name = "python_audit_qualification"' "tools/ci/ci_targets_d.bzl"; then
  if grep -q -F -e 'bazel run --noshow_progress //tools/ci:python_audit_qualification' "tools/ci/dogfood_freshness.sh" ||
    grep -q -F -e 'bazel run --noshow_progress //tools/ci:python_audit_qualification' tools/ci/dogfood_freshness.sh; then
    ok
  else
    bad "dogfood_freshness.sh lost the python_audit_qualification wiring"
  fi
else
  bad "tools/ci/BUILD.bazel or ci_targets_d.bzl lost the python_audit_qualification target"
fi

# Fixture expected covers the selection with owned gaps.
if grep -q -F -e 'qualified seed-only under issue #801' "$expected" &&
  grep -q -F -e 'Successor to closed #613' "$expected" &&
  grep -q -F -e 'Ruff S' "$expected" &&
  grep -q -F -e 'Bandit excluded' "$expected" &&
  grep -q -F -e 'owned gaps' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "python_audit.expected lost selection coverage (want qualified plus successor plus Ruff S plus Bandit plus owned gaps plus no Supported, issue #801)"
fi

# Audit samples pin the S ruleset split (clean versus S101 plus S602).
if grep -q -F -e 'assert' "$dirty_sample" &&
  grep -q -F -e 'shell=True' "$dirty_sample" &&
  ! grep -q -F -e 'assert' "$clean_sample"; then
  ok
else
  bad "audit samples lost their S ruleset split (want assert plus shell=True dirty, clean without assert)"
fi

# Platform evidence: Ruff per-host artifacts stay present (macOS x86_64
# removed per #976).
if [[ -f "quality/artifacts/ruff.linux_x86_64.bzl" && -f "quality/artifacts/ruff.linux_arm64.bzl" && -f "quality/artifacts/ruff.macos_arm64.bzl" && -f "quality/artifacts/ruff.windows_x86_64.bzl" && ! -f "quality/artifacts/ruff.macos_x86_64.bzl" ]]; then
  ok
else
  bad "Ruff per-host artifacts missing (want quality/artifacts/ruff per-host bzl for platform evidence)"
fi

# Consumer plus release evidence stays linked.
if [[ -d "$adopt" ]] &&
  grep -q -F -e 'promotion to `Supported` requires release evidence' "$promotion"; then
  ok
else
  bad "consumer plus release evidence missing (want examples/adopt-python plus support-matrix release-evidence gate)"
fi

# Verification-remaining mirrors the #801 record.
# Live proof: the fixture build stays green on the seed host.
if bazel build //python/tests/fixtures/python_audit/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "python audit live proof failed (want //python/tests/fixtures/python_audit green, issue #801)"
fi

dx_test_summary "python audit qualification harness"
