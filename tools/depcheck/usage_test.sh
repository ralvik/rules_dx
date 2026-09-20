#!/usr/bin/env bash
# Declared-dependency usage test driver (; opens under).
# Usage: usage_test.sh <ecosystem> <depcheck.py> <testdata-root>
# Verifies usage truth table, transitive/shared, non-import exceptions
# with reasons, obsolete, platform/optional, category, non-mutating,
# offline halves for one language.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

eco="$1"
checker_in="$2"
root_in="$3"

# `resolve` removed: use dx_resolve_runfile from tools/sh/lib.sh.
checker="$(dx_resolve_runfile "$checker_in")" || {
  echo "FAIL: cannot resolve $checker_in" >&2
  exit 1
}
root="$(dx_resolve_runfile "$root_in")" || {
  echo "FAIL: cannot resolve $root_in" >&2
  exit 1
}

dx_test_init

run_use() {
  # $1 manifest, $2 sources, $3 exceptions (optional)
  if [[ -n "${3:-}" ]]; then
    python3 "$checker" usage --ecosystem "$eco" --manifest "$1" --sources "$2" --exceptions "$3"
  else
    python3 "$checker" usage --ecosystem "$eco" --manifest "$1" --sources "$2"
  fi
}

case "$eco" in
  rust) man="Cargo.toml" ;;
  python) man="pyproject.toml" ;;
  js | ts) man="package.json" ;;
  go) man="go.mod" ;;
  java | kotlin | scala) man="jvm_deps.toml" ;;
  csharp | fsharp) man="paket.dependencies" ;;
  cc) man="cc_deps.toml" ;;
  *)
    echo "unknown ecosystem $eco" >&2
    exit 2
    ;;
esac

srcdir() {
  # sources dir for a case: rust uses src/ or crates/, python src/ or
  # pkgs, js/ts src/ or packages/. Pass the case root; checker scans
  # recursively across the owning scope.
  echo "$root/$1"
}

# ok_used passes (all declarations used, transitive ignored).
if run_use "$root/ok_used/$man" "$(srcdir ok_used)" >/dev/null; then ok "$eco all-used passes"; else bad "$eco ok_used should pass"; fi

# stale sources (all used) pass usage even though consistency fails.
if run_use "$root/stale/$man" "$(srcdir stale)" >/dev/null; then ok "$eco stale-used passes usage"; else bad "$eco stale should pass usage"; fi

# consistent+unused fails usage.
if run_use "$root/unused/$man" "$(srcdir unused)" >/dev/null; then bad "$eco unused should fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco consistent+unused fails usage"; else bad "$eco unused exit=$code want 1"; fi
fi

# transitive + shared-workspace passes (transitive ignored, cross-use counts).
if run_use "$root/transitive_shared/$man" "$(srcdir transitive_shared)" >/dev/null; then ok "$eco transitive+shared passes"; else bad "$eco transitive_shared should pass usage"; fi

# non-import exception with reason passes.
if run_use "$root/exception/$man" "$(srcdir exception)" "$root/exception/depcheck_exceptions.toml" >/dev/null; then ok "$eco explained exception passes"; else bad "$eco exception should pass"; fi

# unrelated unused still fails even with a valid exception present.
dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/depcheck.XXXXXX"
cp -RL "$root/exception/." "$scratch/" 2>/dev/null || cp -rL "$root/exception/." "$scratch/"
chmod -R u+w "$scratch"
if [[ "$eco" == "rust" ]]; then
  python3 - "$scratch/Cargo.toml" <<'PY'
import sys
p = sys.argv[1]
t = open(p).read()
t = t.replace('[dependencies]\n', '[dependencies]\nunused-extra = "9"\n', 1)
open(p, "w").write(t)
PY
  cat >>"$scratch/Cargo.lock" <<'EOF'

[[package]]
name = "unused-extra"
version = "9.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fixture-extra"
EOF
elif [[ "$eco" == "python" ]]; then
  # append an unused dep to pyproject + lock
  python3 - "$scratch/pyproject.toml" <<'PY'
import sys
p = sys.argv[1]
t = open(p).read()
t = t.replace('dependencies = ["pytest>=7"', 'dependencies = ["pytest>=7", "unused-extra==9.0.0"')
open(p, "w").write(t)
PY
  cat >>"$scratch/uv.lock" <<'EOF'

[[package]]
name = "unused-extra"
version = "9.0.0"
source = { registry = "https://pypi.org/simple" }
EOF
elif [[ "$eco" == "go" ]]; then
  printf '\nrequire example.com/unusedextra v9.0.0\n' >>"$scratch/go.mod"
  cat >>"$scratch/go.sum" <<'EOF'
example.com/unusedextra v9.0.0 h1:fixture-extra-unusedextra-9.0.0
example.com/unusedextra v9.0.0/go.mod h1:fixture-mod-extra-unusedextra
EOF
elif [[ "$eco" == "java" || "$eco" == "kotlin" || "$eco" == "scala" ]]; then
  cat >>"$scratch/jvm_deps.toml" <<'EOF'

[[dep]]
group = "example"
artifact = "unusedextra"
version = "9.0.0"
scope = "compile"
EOF
  python3 - "$scratch/maven_install.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d.setdefault("artifacts", {})["example:unusedextra"] = {"version": "9.0.0", "shasums": {"jar": "fixture-extra"}}
json.dump(d, open(p, "w"), indent=2)
PY
elif [[ "$eco" == "csharp" || "$eco" == "fsharp" ]]; then
  printf '\nnuget UnusedExtra 9.0.0\n' >>"$scratch/paket.dependencies"
  cat >>"$scratch/paket.lock" <<'EOF'
    UnusedExtra (9.0.0)
EOF
elif [[ "$eco" == "cc" ]]; then
  cat >>"$scratch/cc_deps.toml" <<'EOF'

[[dep]]
name = "unused-extra"
version = "9.0.0"
sha256 = "fixture-sha256-unused-extra-9.0.0"
scope = "prod"
EOF
  python3 - "$scratch/cc_lock.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d.setdefault("packages", {})["unused-extra"] = {"version": "9.0.0", "sha256": "fixture-sha256-unused-extra-9.0.0"}
json.dump(d, open(p, "w"), indent=2)
PY
else
  python3 - "$scratch/package.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d.setdefault("dependencies", {})["unused-extra"] = "9.0.0"
json.dump(d, open(p, "w"), indent=2)
PY
  cat >>"$scratch/pnpm-lock.yaml" <<'EOF'
  'unused-extra@9.0.0':
    resolution: {integrity: sha512-fixture-extra}
EOF
fi
if run_use "$scratch/$man" "$scratch" "$scratch/depcheck_exceptions.toml" >/dev/null 2>&1; then bad "$eco unrelated unused should still fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco exception does not suppress unrelated unused"; else bad "$eco extra-unused exit=$code want 1"; fi
fi
rm -rf "$scratch"
mkdir -p "$scratch"

# missing reason fails validation.
dx_mkscratch scratch2 "${TEST_TMPDIR:-/tmp}/depcheck.XXXXXX"
cp -RL "$root/exception/." "$scratch2/" 2>/dev/null || cp -rL "$root/exception/." "$scratch2/"
chmod -R u+w "$scratch2"
case "$eco" in
  go) exc_dep="example.com/buildplugin" ;;
  java | kotlin | scala) exc_dep="example:buildplugin" ;;
  csharp | fsharp) exc_dep="BuildPlugin" ;;
  cc) exc_dep="build-plugin" ;;
  rust | python | js | ts) exc_dep="build-plugin" ;;
  *) exc_dep="build-plugin" ;;
esac
cat >"$scratch2/depcheck_exceptions.toml" <<EOF
[[exception]]
dependency = "$exc_dep"
reason = ""
EOF
if run_use "$scratch2/$man" "$scratch2" "$scratch2/depcheck_exceptions.toml" >/dev/null 2>&1; then bad "$eco missing reason should fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco missing reason fails validation"; else bad "$eco missing-reason exit=$code want 1"; fi
fi
rm -rf "$scratch2"

# obsolete: removed dep + unnecessary exception both fail; still-needed passes elsewhere.
if run_use "$root/obsolete/$man" "$(srcdir obsolete)" "$root/obsolete/depcheck_exceptions.toml" >/dev/null 2>&1; then bad "$eco obsolete should fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco obsolete exceptions fail"; else bad "$eco obsolete exit=$code want 1"; fi
fi
# still-needed explained exception continues to pass (exception fixture above).

# platform/optional used in supported config passes without exceptions.
if run_use "$root/platform_optional/$man" "$(srcdir platform_optional)" >/dev/null; then ok "$eco platform/optional passes without exception"; else bad "$eco platform_optional should pass"; fi

# unused optional/platform-specific still fails (optional is not proof).
if run_use "$root/platform_optional_unused/$man" "$(srcdir platform_optional_unused)" >/dev/null 2>&1; then bad "$eco unused optional should fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco unused optional still fails"; else bad "$eco unused-optional exit=$code want 1"; fi
fi

# category: prod used only by tests fails with a category error.
if run_use "$root/category/$man" "$(srcdir category)" >/dev/null 2>&1; then bad "$eco category should fail"; else
  out="$(run_use "$root/category/$man" "$(srcdir category)" 2>&1 || true)"
  if echo "$out" | grep -q -F -e 'category error'; then ok "$eco prod-only-in-tests fails with category error"; else
    bad "$eco category missing 'category error'"
    echo "$out" >&2
  fi
fi

# correctly categorized + multi-category passes (incl. other-config use).
if run_use "$root/category_ok/$man" "$(srcdir category_ok)" >/dev/null; then ok "$eco correctly categorized + multi passes"; else bad "$eco category_ok should pass"; fi

# diagnostics do not mutate manifests/locks/sources.
# Portable tree digest via dx_tree_sha256.
for case in ok_used unused category; do
  before="$(dx_tree_sha256 "$root/$case")"
  run_use "$root/$case/$man" "$(srcdir "$case")" >/dev/null 2>&1 || true
  after="$(dx_tree_sha256 "$root/$case")"
  if [[ "$before" == "$after" ]]; then ok "$eco $case diagnostics do not mutate"; else bad "$eco $case mutated"; fi
done

# no network imports (offline route); no foreign execution (text scan only).
if grep -rn -E -e 'import urllib|import socket|import http|import requests|from urllib|subprocess|os\.system|os\.exec' "$checker" >/dev/null 2>&1; then bad "$eco checker must stay offline/no-exec"; else ok "$eco offline/no-exec (no network/subprocess imports)"; fi

dx_test_summary "usage $eco"
