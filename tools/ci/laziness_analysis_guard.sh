#!/usr/bin/env bash
# Laziness analysis guard: zero-delta fetches/actions plus analysis budget.
#
# ADR 0014 plus the architecture README promise unused foundations add no
# configured targets, actions, toolchain payload, env projection, download,
# or fetch. Slices 2-5 prove shape once (static isolation, query deps,
# aquery markers, action commands); this guard proves cost continuously:
# adding a bazel_dep+extension must not silently add configured
# targets/actions/fetches to single-foundation adopt-* consumers or creep
# analysis time linearly, with no red test.
#
# Matrix (10 single-foundation consumers, adopt-polyglot out
# multi-foundation; cc/java negative-only, same rule as slices 3/4):
# adopt-rust, adopt-python, adopt-js-ts, adopt-go, adopt-cpp, adopt-java,
# adopt-kotlin, adopt-scala, adopt-csharp, adopt-fsharp.
#
# Metrics (all pinned in tools/ci/tests/fixtures/laziness_analysis/pins.bzl):
# - M1 configured targets: cquery deps() --output=label | sort -u | wc -l
#   equals pin, delta 0 (unconfigured query is blind to select() and
#   toolchain resolution, so configured cquery is required).
# - M2 actions: aquery --output=text | grep -c ActionKey: equals pin, plus
#   forbidden-marker absence (same markers as slices 3/4, never
#   rules_cc/rules_java/bazel_tools/skylib/platforms).
# - M3 fetch set: cquery deps() repo prefixes (Bzlmod portable equivalent
#   of the resolved file, absent on Bazel 9.2.0) subset of the per-consumer
#   allowlist, plus an isolated-cache spot check (one example, rotating).
#   Set equality only, never cold bytes (stays with cold-server bytes).
# - M4 analysis time: build --nobuild --profile runAnalysisPhase duration
#   within budget (pin plus generous headroom, uniform 30s); counts hard
#   fail, time is informational-regression (noisy-neighbor rerun
#   legitimate) per no-benchmarking (counts gate, time is capped budget).
#
# Seed-only, warm cache, budget under 5 minutes. No new job; runs in
# dogfood-freshness after examples_laziness_runtime, before
# quality_cache_aquery. Analysis-only (--nobuild, no execution, no remote
# flags); portable scratch via TMPDIR/RUNNER_TEMP plus mktemp, no hardcode.
#
# Versioned here, run by CI via `bazel run //tools/ci:laziness_analysis_guard`,
# after //tools/ci:examples_laziness_runtime.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="tools/ci/tests/fixtures/laziness_analysis/pins.bzl"
pins_build="tools/ci/tests/fixtures/laziness_analysis/BUILD.bazel"
expected="tools/ci/tests/fixtures/laziness_analysis/laziness_analysis.expected"
ci_targets="tools/ci/ci_targets_a.bzl"
dogfood="tools/ci/dogfood_freshness.sh"
tools_doc="docs/testing/tools.md"

# Fixture files stay present with the four metric pins.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]] &&
  grep -q -F -e 'CQUERY_EXPECTED' "$pins" &&
  grep -q -F -e 'AQUERY_EXPECTED' "$pins" &&
  grep -q -F -e 'ANALYSIS_BUDGET_MS' "$pins" &&
  grep -q -F -e 'FETCH_ALLOWLIST' "$pins"; then
  ok
else
  bad "laziness_analysis fixture missing (want pins.bzl plus BUILD.bazel plus expected with M1/M2/M3/M4 pins)"
fi

# Pins carry all ten consumers with zero-delta counts plus allowlists.
if grep -q -F -e '"adopt-rust": 485' "$pins" &&
  grep -q -F -e '"adopt-python": 2405' "$pins" &&
  grep -q -F -e '"adopt-js-ts": 6109' "$pins" &&
  grep -q -F -e '"adopt-go": 6103' "$pins" &&
  grep -q -F -e '"adopt-cpp": 174' "$pins" &&
  grep -q -F -e '"adopt-rust": 92' "$pins" &&
  grep -q -F -e '"adopt-python": 126' "$pins" &&
  grep -q -F -e 'ANALYSIS_BUDGET_MS' "$pins" &&
  grep -q -F -e 'FETCH_ALLOWLIST' "$pins"; then
  ok
else
  bad "pins.bzl lost its ten-consumer cquery/aquery pins plus budgets plus allowlists"
fi

# Wiring stays ordered in dogfood-freshness: runtime, then this guard,
# then quality-cache aquery (no new job).
if grep -q -F -e '//tools/ci:laziness_analysis_guard' "$dogfood" &&
  grep -q -F -e '//tools/ci:examples_laziness_runtime' "$dogfood" &&
  grep -q -F -e '//tools/ci:quality_cache_aquery' "$dogfood"; then
  runtime_line="$(grep -n -F -e 'examples_laziness_runtime' "$dogfood" | head -1 | cut -d: -f1)"
  guard_line="$(grep -n -F -e 'laziness_analysis_guard' "$dogfood" | head -1 | cut -d: -f1)"
  cache_line="$(grep -n -F -e 'quality_cache_aquery' "$dogfood" | head -1 | cut -d: -f1)"
  if [[ "$runtime_line" -lt "$guard_line" && "$guard_line" -lt "$cache_line" ]]; then
    ok
  else
    bad "dogfood_freshness lost its runtime-guard-cache order (want runtime < guard < cache)"
  fi
else
  bad "dogfood_freshness lost its laziness_analysis_guard wiring (want runtime then guard then cache)"
fi

# Target stays Linux-only per the shell contract.
if grep -q -F -e 'laziness_analysis_guard' "$ci_targets" &&
  grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' "$ci_targets"; then
  ok
else
  bad "ci_targets_a.bzl lost its Linux-only laziness_analysis_guard target"
fi

# Docs own the guard: tools matrix plus verification battery.
if grep -q -F -e 'laziness_analysis_guard' "$tools_doc" &&
  grep -q -F -e 'laziness' "$tools_doc"; then
  ok
else
  bad "docs/testing/tools.md lost its laziness_analysis_guard link"
fi
# Single-source pin readers (pins.bzl is valid Python for exec).
pin_cquery() {
  python3 -c "ns={}; exec(open('$pins').read(), ns); print(ns['CQUERY_EXPECTED']['$1'])"
}
pin_aquery() {
  python3 -c "ns={}; exec(open('$pins').read(), ns); print(ns['AQUERY_EXPECTED']['$1'])"
}
pin_budget() {
  python3 -c "ns={}; exec(open('$pins').read(), ns); print(ns['ANALYSIS_BUDGET_MS']['$1'])"
}
pin_allowlist() {
  python3 -c "ns={}; exec(open('$pins').read(), ns); print(' '.join(ns['FETCH_ALLOWLIST']['$1']))"
}
pin_forbidden() {
  local ex="$1" var=""
  case "$ex" in
    adopt-rust) var="FORBIDDEN_RUST" ;;
    adopt-python) var="FORBIDDEN_PYTHON" ;;
    adopt-js-ts) var="FORBIDDEN_JS" ;;
    adopt-go) var="FORBIDDEN_GO" ;;
    adopt-csharp | adopt-fsharp) var="FORBIDDEN_DOTNET" ;;
    adopt-kotlin) var="FORBIDDEN_KOTLIN" ;;
    adopt-scala) var="FORBIDDEN_SCALA" ;;
    *) var="FORBIDDEN_NEGATIVE" ;;
  esac
  python3 -c "ns={}; exec(open('$pins').read(), ns); print(' '.join(ns['$var']))"
}
want_marker() {
  case "$1" in
    adopt-rust) printf 'rules_rust' ;;
    adopt-python) printf 'aspect_rules_py' ;;
    adopt-js-ts) printf 'aspect_rules_js' ;;
    adopt-go) printf 'rules_go' ;;
    adopt-csharp | adopt-fsharp) printf 'rules_dotnet' ;;
    adopt-kotlin) printf 'rules_kotlin' ;;
    adopt-scala) printf 'rules_scala' ;;
    *) printf '' ;;
  esac
}
# Repo base normalization: @@module++ext+repo -> module, @repo -> repo,
# empty module (root extension) -> main. Mirrors pins.bzl derivation.
repo_bases() {
  python3 -c '
import sys
repos = set()
for line in open(sys.argv[1]):
    label = line.strip().split(" ")[0]
    if label.startswith("@@"):
        repos.add(label.split("//")[0])
    elif label.startswith("@"):
        repos.add(label.split("//")[0].split(":")[0])
bases = set()
for r in repos:
    if r.startswith("@@"):
        inner = r[2:]
        mod = inner.split("++")[0].split("+")[0]
        if not mod:
            parts = inner.split("+")
            mod = next((p for p in parts if p), "main")
            if mod == "main_extension":
                mod = "main"
        bases.add(mod)
    else:
        inner = r[1:]
        mod = inner.split("+")[0].split("@")[0]
        if "__" in mod:
            bases.add("npm")
        else:
            bases.add(mod)
for b in sorted(bases):
    print(b)
' "$1"
}
profile_ms() {
  python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
cands = [e for e in d.get("traceEvents", []) if e.get("name") == "runAnalysisPhase"]
if not cands:
    print(-1)
else:
    print(cands[0].get("dur", -1) // 1000)
' "$1"
}

# Fresh server once so configured counts prove analysis shape, not warmth.
bazel shutdown >/dev/null 2>&1 || true

dx_mkscratch laziness_scratch "${TMPDIR:-${RUNNER_TEMP:-/tmp}}/laziness-analysis.XXXXXX"

examples="adopt-rust adopt-python adopt-js-ts adopt-go adopt-cpp adopt-java adopt-kotlin adopt-scala adopt-csharp adopt-fsharp"

for ex in $examples; do
  # M1 configured-target count plus M3 repo set share one cquery.
  cquery_out="$laziness_scratch/cquery-$ex.txt"
  if ! bazel cquery "deps(//examples/$ex/...)" --output=label --noshow_progress >"$cquery_out" 2>/dev/null; then
    bad "$ex: bazel cquery failed"
    continue
  fi
  got_cquery="$(sort -u "$cquery_out" | wc -l | tr -d ' ')"
  want_cquery="$(pin_cquery "$ex")"
  if [[ "$got_cquery" == "$want_cquery" ]]; then
    ok
  else
    bad "$ex: configured-target delta (want $want_cquery, got $got_cquery; offending labels: $(sort -u "$cquery_out" | head -n 3 | tr '\n' ' '))"
  fi

  # M2 action count plus forbidden markers share one aquery.
  aquery_out="$laziness_scratch/aquery-$ex.txt"
  if ! bazel aquery "//examples/$ex/..." --output=text --noshow_progress >"$aquery_out" 2>/dev/null; then
    bad "$ex: bazel aquery failed"
    continue
  fi
  if [[ ! -s "$aquery_out" ]]; then
    bad "$ex: empty aquery output"
    continue
  fi
  got_aquery="$(grep -c 'ActionKey:' "$aquery_out" || true)"
  want_aquery="$(pin_aquery "$ex")"
  if [[ "$got_aquery" == "$want_aquery" ]]; then
    ok
  else
    bad "$ex: action-count delta (want $want_aquery, got $got_aquery)"
  fi
  want="$(want_marker "$ex")"
  if [[ -n "$want" ]]; then
    if grep -q -F -e "$want" "$aquery_out"; then
      ok
    else
      bad "$ex: want marker [$want] in aquery actions"
    fi
  fi
  for marker in $(pin_forbidden "$ex"); do
    if grep -q -F -e "$marker" "$aquery_out"; then
      bad "$ex: forbidden marker [$marker] in aquery actions (unused foundation leaks)"
    else
      ok
    fi
  done

  # M3 fetch subset: every repo base must be allowlisted. Allowlist alone
  # owns the fetch gate: configured cquery legitimately resolves
  # base-toolchain pulls (e.g. aspect_rules_py into java/kotlin/scala)
  # that stay allowlisted, so no separate forbidden list here.
  allow="$(pin_allowlist "$ex")"
  bases="$(repo_bases "$cquery_out")"
  for base in $bases; do
    case " $allow " in
      *" $base "*) ok ;;
      *) bad "$ex: forbidden fetch base [$base] (allowlist: $allow)" ;;
    esac
  done

  # M4 analysis budget: build --nobuild with profile, runAnalysisPhase only.
  profile="$laziness_scratch/profile-$ex.json"
  if ! bazel build "//examples/$ex/..." --nobuild --profile="$profile" --noshow_progress >/dev/null 2>&1; then
    bad "$ex: bazel build --nobuild failed"
    continue
  fi
  got_ms="$(profile_ms "$profile")"
  want_ms="$(pin_budget "$ex")"
  if [[ "$got_ms" == "-1" ]]; then
    bad "$ex: missing runAnalysisPhase in profile"
  elif [[ "$got_ms" -le "$want_ms" ]]; then
    ok
  else
    bad "$ex: analysis budget exceeded (want <= ${want_ms}ms, got ${got_ms}ms; noisy-neighbor rerun legitimate, counts still hard fail)"
  fi
done

# M3 isolated-cache spot check (one example, rotating by day-of-year).
day="$(date +%j | sed 's/^0*//')"
if [[ -z "$day" ]]; then
  day="1"
fi
idx="$((day % 10))"
spot="$(printf '%s' "$examples" | tr ' ' '\n' | sed -n "$((idx + 1))p")"
if [[ -z "$spot" ]]; then
  spot="adopt-rust"
fi
dx_mkscratch repo_cache "${TMPDIR:-${RUNNER_TEMP:-/tmp}}/laziness-repo-cache.XXXXXX"
if bazel build "//examples/$spot/..." --nobuild --repository_cache="$repo_cache" --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "spot $spot: isolated-cache build --nobuild failed"
fi
cache_hits="$(find "$repo_cache" -maxdepth 3 -print 2>/dev/null | grep -i -e 'dotnet' -e 'maven' -e 'pypi' -e 'npm' || true)"
if [[ -z "$cache_hits" ]]; then
  ok
else
  bad "spot $spot: forbidden fetch in isolated cache: $(printf '%s' "$cache_hits" | head -n 3 | tr '\n' ' ')"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/laziness_analysis/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "laziness_analysis fixture failed to build (want green on the seed host)"
fi

dx_test_summary "laziness analysis guard"
