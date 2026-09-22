#!/usr/bin/env bash
# Docs/testing gates for issue #926 (items 1,2,3,4,7,8-env).
#
# Machine-checks the docs/testing drift that previously had no continuous
# gate: README length with grandfathered split plan, root-vs-docs AGENTS
# sync, in-code-docs exact rg plus Contract/See pins, markers-only Vale
# pin with prose-pass proof, dogfood-freshness parity (sh == Battery ==
# ci.yml step), and the CI UPDATE_EXPECT env guard.
#
# Versioned here, run by CI via `bazel run //tools/ci:docs_testing_gates`,
# following //tools/ci:source_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# --- Item 1: README length (60 lines, splits completed under #986) ---
limit="60"
split_plan="docs/testing/readme-split-plan.md"
if [[ -f "$split_plan" ]] &&
  grep -q -F -e 'docs/testing/README.md' "$split_plan" &&
  grep -q -F -e 'docs/architecture/README.md' "$split_plan"; then
  ok
else
  bad "$split_plan missing or lost its testing/architecture split record (want completed splits with plan, issues #926/#986)"
fi
# Splits completed under #986: no over-limit README remains, so the allowlist
# stays empty. New READMEs must stay at/under the limit.
allowlisted=()
over_bad=""
while IFS= read -r f; do
  rel="${f#./}"
  n="$(wc -l <"$f" | tr -d ' ')"
  listed=0
  for a in "${allowlisted[@]}"; do
    if [[ "$rel" == "$a" ]]; then
      listed=1
      break
    fi
  done
  if [[ "$n" -gt "$limit" && "$listed" == "0" ]]; then
    over_bad="$over_bad $rel:$n"
  fi
done < <(find . -name "README.md" -not -path "./bazel-*" -not -path "./.git/*" | LC_ALL=C sort)
if [[ -z "$over_bad" ]]; then
  ok
else
  bad "README length exceeded ${limit} lines:$over_bad (want short READMEs with details in docs/ or examples/, issues #926/#986)"
fi
# Allowlist stays empty now that splits completed under #986; any future
# over-limit README fails above without an allowlist entry.
for a in "${allowlisted[@]}"; do
  if [[ -f "$a" ]] && grep -q -F -e "$a" "$split_plan"; then
    ok
  else
    bad "allowlist entry $a missing from tree or split plan (want grandfather with plan, issue #926)"
  fi
done
# Examples stay short by construction (adopt-* teach plus prove in ~35 lines).
examples_bad=""
for dir in examples/adopt-*/; do
  readme="$dir/README.md"
  if [[ -f "$readme" ]]; then
    n="$(wc -l <"$readme" | tr -d ' ')"
    if [[ "$n" -gt "$limit" ]]; then
      examples_bad="$examples_bad $readme:$n"
    fi
  fi
done
if [[ -z "$examples_bad" ]]; then
  ok
else
  bad "adopt-* READMEs exceeded ${limit} lines:$examples_bad (want short consumer docs, issue #926)"
fi

# --- Item 2: root-vs-docs AGENTS sync guard ---
if grep -q -F -e 'see `docs/AGENTS.md`' AGENTS.md &&
  grep -q -F -e 'Treat warnings as errors' AGENTS.md &&
  grep -q -F -e 'Keep READMEs short' AGENTS.md &&
  grep -q -F -e 'In-code docs' AGENTS.md; then
  ok
else
  bad "AGENTS.md lost its root summary plus docs pointer (want Bazel plus warnings plus short-READMEs plus in-code-docs with see docs/AGENTS.md, issue #926)"
fi
if grep -q -F -e "Link-don't-copy applies to code comments" docs/AGENTS.md &&
  grep -q -F -e '.bzl` headers carry one-line purpose' docs/AGENTS.md &&
  grep -q -F -e 'No bare `Issue #`' docs/AGENTS.md; then
  ok
else
  bad "docs/AGENTS.md lost its enforceable code-comment rule (want link-don't-copy plus .bzl header plus bare-Issue rule, issue #926)"
fi
# Root summary phrases stay owned by docs (sync, not copy): Contract, See,
# LCOV_EXCL, and Issue tokens appear in both files.
if grep -q -F -e 'Contract:' AGENTS.md &&
  grep -q -F -e 'Contract:' docs/AGENTS.md &&
  grep -q -F -e 'LCOV_EXCL' AGENTS.md &&
  grep -q -F -e 'LCOV_EXCL' docs/AGENTS.md &&
  grep -q -F -e 'Issue #' AGENTS.md &&
  grep -q -F -e 'Issue #' docs/AGENTS.md; then
  ok
else
  bad "AGENTS root/docs sync drifted (want Contract plus LCOV_EXCL plus Issue tokens in both, issue #926)"
fi
# source_hygiene stays pinned to the enforceable docs rule.
if grep -q -F -e 'docs/AGENTS.md' tools/ci/source_hygiene.sh; then
  ok
else
  bad "source_hygiene.sh lost its docs/AGENTS.md pin (want enforceable-rule owner, issue #926)"
fi

# --- Item 3: in-code-docs gate (exact rg plus Contract plus Rust See) ---
dx_mkscratch incode_scratch
rg "Issue #|issue #" --glob '*.rs' --glob '*.bzl' cli/ quality/ tools/ --line-number >"$incode_scratch/rg.txt" || true
if python3 - "$incode_scratch/rg.txt" <<'EOF'
import sys
viol = []
for line in open(sys.argv[1], errors="ignore"):
    line = line.rstrip("\n")
    if not line:
        continue
    path = line.split(":", 1)[0]
    text = line
    if "pins.bzl" in path or "_tests" in path or "/tests/" in path or "/testdata/" in path:
        continue
    if "See:" in text or "Owning contract:" in text:
        continue
    viol.append(line)
print(f"bare-Issue violations: {len(viol)}")
for v in viol[:20]:
    print(v)
sys.exit(1 if viol else 0)
EOF
then
  ok
else
  bad "bare Issue refs outside See:/Owning contract:/test-pins (run: rg \"Issue #|issue #\" --glob '*.rs' --glob '*.bzl' cli/ quality/ tools/, want only See:/Owning-contract plus pins.bzl/*_tests*/tests//testdata/, issue #926)"
fi
# Flagship .bzl headers keep one-line purpose plus owning-contract link.
if grep -q -F -e 'Contract:' generation/codegen.bzl &&
  grep -q -F -e 'Contract:' quality/adapters.bzl &&
  grep -q -F -e 'Contract:' libs/starlark/wrapper.bzl &&
  grep -q -F -e 'Contract:' env/plan.bzl; then
  ok
else
  bad "flagship .bzl headers lost owning-contract links (want codegen plus adapters plus wrapper plus env, issue #926)"
fi
# Rust keeps only why-not-obvious notes with See:/Owning contract: links;
# the #710 audit was genealogy-only and stays deleted per #982.
if [[ ! -f "docs/documentation/in-code-docs-audit.md" ]] &&
  ! grep -q -F -e 'docs/documentation/in-code-docs-audit.md' docs/AGENTS.md &&
  ! grep -q -F -e '#710' docs/AGENTS.md; then
  ok
else
  bad "in-code-docs audit genealogy reappeared (want audit deleted per #982, issue #926)"
fi

# --- Item 4: markers-only Vale pin (prose passes, prose rules rejected) ---
if grep -q -F -e 'BasedOnStyles = Dx' quality/corpus_vale.ini &&
  grep -q -F -e 'StylesPath = corpus_styles' quality/corpus_vale.ini &&
  grep -q -F -e 'MinAlertLevel = suggestion' quality/corpus_vale.ini; then
  ok
else
  bad "quality/corpus_vale.ini lost its Dx binding (want StylesPath plus BasedOnStyles = Dx plus MinAlertLevel, issue #926)"
fi
if grep -q -F -e 'extends: existence' quality/corpus_styles/Dx/Markers.yml &&
  grep -q -F -e 'TODO' quality/corpus_styles/Dx/Markers.yml &&
  grep -q -F -e 'FIXME' quality/corpus_styles/Dx/Markers.yml &&
  grep -q -F -e 'XXX' quality/corpus_styles/Dx/Markers.yml; then
  ok
else
  bad "corpus_styles/Dx/Markers.yml lost its markers-only existence rule (want TODO plus FIXME plus XXX, issue #926)"
fi
# Markers-only stays markers-only: no prose/spelling/style rule may ride Dx.
if ! grep -E -e 'extends:[[:space:]]*(spelling|prose|style|readability|vale\.|proselint|write-good|alex)' quality/corpus_styles/Dx/Markers.yml >/dev/null &&
  ! grep -F -e 'Vale.Spelling' quality/corpus_styles/Dx/Markers.yml >/dev/null &&
  ! grep -F -e 'Vale.Prose' quality/corpus_styles/Dx/Markers.yml >/dev/null; then
  ok
else
  bad "Dx.Markers gained a prose rule (want markers-only existence, prose stays wont-fix, issue #926)"
fi
# Negative proof: prose-heavy docs with no markers pass the markers-only
# policy by construction (markers grep finds nothing to flag).
dx_mkscratch markers_scratch
cat >"$markers_scratch/prose.md" <<'EOF'
# Prose fixture

This documentation sentence is deliberately verbose and utilizes passive
voice constructions that any prose linter would flag as utterly terrible
style with excessive adverbs and very long winding clauses that go on and
on without ever reaching a particularly useful conclusion whatsoever.
EOF
if ! grep -E -e 'TODO|FIXME|XXX' "$markers_scratch/prose.md" >/dev/null; then
  ok
else
  bad "prose fixture unexpectedly carries markers (want marker-free prose proving markers-only passes prose, issue #926)"
fi
# Strict stays markers-only too (no hidden prose preset).
if grep -q -F -e 'Vale strict stays markers-only' docs/quality/strict-preset.md; then
  ok
else
  bad "docs/quality/strict-preset.md lost its markers-only strict record (want prose wont-fix, issue #926)"
fi

# --- Item 7: dogfood-freshness parity (sh == Battery == ci.yml step) ---
dogfood="tools/ci/dogfood_freshness.sh"
ci=".github/workflows/ci.yml"
if [[ -f "$dogfood" ]]; then
  ok
else
  bad "tools/ci/dogfood_freshness.sh missing (want versioned battery, issue #926)"
fi
# Every `bazel run //tools/ci:<name>` in the battery script is owned by a
# matching sh_binary/sh_test target in the ci_targets shards.
dx_mkscratch parity_scratch
grep -o -E -e '//tools/ci:[a-z0-9_]+' "$dogfood" | sed 's|//tools/ci:||' | LC_ALL=C sort -u >"$parity_scratch/dogfood_targets.txt"
grep -h -o -E -e 'name = "[a-z0-9_]+"' tools/ci/ci_targets_a.bzl tools/ci/ci_targets_b.bzl tools/ci/ci_targets_c.bzl tools/ci/ci_targets_d.bzl | sed 's/name = //; s/"//g' | LC_ALL=C sort -u >"$parity_scratch/build_targets.txt"
parity_missing=""
while IFS= read -r t; do
  if ! grep -q -F -e "$t" "$parity_scratch/build_targets.txt"; then
    parity_missing="$parity_missing $t"
  fi
done <"$parity_scratch/dogfood_targets.txt"
if [[ -z "$parity_missing" ]]; then
  ok
else
  bad "dogfood-freshness lists unowned targets:$parity_missing (want every //tools/ci:<name> owned in ci_targets shards, issue #926)"
fi
# New #926 gates ride the battery (no silent additions outside dogfood).
if grep -q -F -e '//tools/ci:docs_testing_gates' "$dogfood" &&
  grep -q -F -e '//tools/ci:examples_consumer_gates' "$dogfood"; then
  ok
else
  bad "dogfood-freshness lost the #926 gates (want //tools/ci:docs_testing_gates plus //tools/ci:examples_consumer_gates, issue #926)"
fi
# CI runs the battery through the Bazel-owned target, never direct sh.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:dogfood_freshness' "$ci" &&
  ! grep -F -e 'run: tools/ci/dogfood_freshness.sh' "$ci" | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  ok
else
  bad "ci.yml dogfood-freshness step drifted (want bazel run //tools/ci:dogfood_freshness, never direct sh, issue #926)"
fi
# BUILD owns the battery script itself as a runnable target.
if grep -q -F -e 'name = "dogfood_freshness"' tools/ci/ci_targets_d.bzl; then
  ok
else
  bad "tools/ci BUILD shards lost the dogfood_freshness sh_binary (want Bazel-owned battery, issue #926)"
fi

# --- Item 8 (env half): CI never sets UPDATE_EXPECT=1 ---
if ! grep -F -e 'UPDATE_EXPECT=1' "$ci" | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  ok
else
  bad "ci.yml sets UPDATE_EXPECT=1 (goldens would auto-pass; want mismatch fails closed, issue #926)"
fi
if grep -q -F -e 'UPDATE_EXPECT=1' docs/testing/strategy-details.md &&
  grep -q -F -e 'UPDATE_EXPECT' tools/sh/snapshot.sh; then
  ok
else
  bad "snapshot refresh workflow lost its docs plus helper pins (want UPDATE_EXPECT in docs/testing/strategy-details.md plus tools/sh/snapshot.sh, issue #926)"
fi

dx_test_summary "docs testing gates (issue #926)"
