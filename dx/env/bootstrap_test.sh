#!/usr/bin/env bash
# M11 WP2 end-to-end bootstrap proof through the real `env` binary:
# fresh install into a workspace whose path contains spaces, second-run
# noop, replacement with stale-entry removal, unmanaged-tree refusal,
# marker presence, and doctor execution via the installed link. Swap
# atomicity and crash recovery are unit-tested in `src/lib.rs`; this test
# proves the runfiles-located default tree and the installed surface
# under Bazel.
set -euo pipefail

env_bin="$(realpath "$1")"
runfiles="${RUNFILES_DIR:-$TEST_SRCDIR}"
root="${TEST_TMPDIR}/work space"
mkdir -p "$root"

output="$("${env_bin}" --workspace "${root}")"
[[ "${output}" == *'installed 5 tool(s)'* ]] || {
  echo "unexpected fresh-install output: ${output}" >&2
  exit 1
}

for tool in doctor dx quality_runner quality_evaluator quality_markdown; do
  [[ -L "${root}/.dx/bin/${tool}" ]] || {
    echo "missing managed symlink: ${tool}" >&2
    exit 1
  }
done
[[ -f "${root}/.dx/bin/.rules_dx_managed" && ! -L "${root}/.dx/bin/.rules_dx_managed" ]] || {
  echo "missing regular marker file" >&2
  exit 1
}
[[ ! -e "${root}/.dx/bin.next" && ! -e "${root}/.dx/bin.prev" ]] || {
  echo "staging leftovers after commit" >&2
  exit 1
}

doctor_out="$("${root}/.dx/bin/doctor")"
[[ "${doctor_out}" == *'rules_dx managed environment tool: //env:doctor'* ]] || {
  echo "unexpected doctor output: ${doctor_out}" >&2
  exit 1
}

noop="$("${env_bin}" --workspace "${root}")"
[[ "${noop}" == *'already current'* ]] || {
  echo "second run was not a noop: ${noop}" >&2
  exit 1
}

# Replacement with a reduced staged tree: the dropped tool's link is
# stale state and must disappear with the swap.
staged="$(find "${runfiles}" -name default_tree.metadata.json -print -quit)"
[[ -n "${staged}" ]] || {
  echo "default staged metadata not found under ${runfiles}" >&2
  exit 1
}
alt="${TEST_TMPDIR}/staged-alt"
mkdir -p "${alt}"
cp -a "$(dirname "${staged}")/bin" "${alt}/bin"
command -v python3 >/dev/null || {
  echo "python3 required to derive the reduced tree" >&2
  exit 1
}
python3 - "${staged}" "${alt}/metadata.json" <<'EOF'
import json
import sys
source, dest = sys.argv[1], sys.argv[2]
with open(source) as handle:
    doc = json.load(handle)
doc["tools"] = [t for t in doc["tools"] if t["bin_name"] != "quality_markdown"]
with open(dest, "w") as handle:
    json.dump(doc, handle)
EOF
rm "${alt}/bin/quality_markdown"
replace="$("${env_bin}" --workspace "${root}" --staged-bin "${alt}/bin" --metadata "${alt}/metadata.json")"
[[ "${replace}" == *'replaced managed tree with 4 tool(s)'* ]] || {
  echo "unexpected replacement output: ${replace}" >&2
  exit 1
}
[[ ! -e "${root}/.dx/bin/quality_markdown" ]] || {
  echo "stale entry survived replacement" >&2
  exit 1
}
[[ -L "${root}/.dx/bin/doctor" ]] || {
  echo "surviving entry lost in replacement" >&2
  exit 1
}

# Restoring the default tree replaces back to the full set.
restore="$("${env_bin}" --workspace "${root}")"
[[ "${restore}" == *'replaced managed tree with 5 tool(s)'* ]] || {
  echo "unexpected restore output: ${restore}" >&2
  exit 1
}
[[ -L "${root}/.dx/bin/quality_markdown" ]] || {
  echo "restored entry missing" >&2
  exit 1
}

# A foreign `.dx/bin` is never adopted and never modified.
foreign="${TEST_TMPDIR}/foreign"
mkdir -p "${foreign}/.dx/bin"
printf 'stale' > "${foreign}/.dx/bin/stale_tool"
set +e
refused="$("${env_bin}" --workspace "${foreign}" 2>&1)"
status=$?
set -e
[[ ${status} -ne 0 ]] || {
  echo "foreign tree was adopted" >&2
  exit 1
}
[[ "${refused}" == *'refusing to touch unmanaged'* ]] || {
  echo "missing unmanaged diagnostic: ${refused}" >&2
  exit 1
}
[[ "$(cat "${foreign}/.dx/bin/stale_tool")" == "stale" ]] || {
  echo "foreign tree was modified" >&2
  exit 1
}
