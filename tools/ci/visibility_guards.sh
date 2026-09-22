#!/usr/bin/env bash
# Visibility and package-boundary guard.
#
# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
# The single contract is `tools/visibility/visibility.bzl`: every package
# default and cross-layer rule lives there, and BUILD files load its scope
# constants by name instead of re-spelling allowlists. See
# docs/contributing/build-conventions.md#visibility.
#
# This harness machine-checks the tree against that contract without
# rebuilding anything:
# - public/private defaults match the contract tables exactly,
# - every scoped package loads and uses its contracted scope constant,
# - no ad-hoc multi-entry visibility literal exists anywhere,
# - explicit public targets stay the contracted binaries/facade/tool,
# - every visibility list is buildifier-canonical (sorted, double-quoted),
# - no declared BUILD edge crosses the dx/cli/tools layering (depcheck leg).
#
# Versioned here, run by CI via `bazel run //tools/ci:visibility_guards`.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

VIS_BZL="tools/visibility/visibility.bzl"

# Canonical doc owns the visibility policy and points at the single contract.
if grep -q -F -e '## Visibility' docs/contributing/build-conventions.md &&
  grep -q -F -e '//tools/ci:visibility_guards' docs/contributing/build-conventions.md &&
  grep -q -F -e 'tools/visibility/visibility.bzl' docs/contributing/build-conventions.md &&
  grep -q -F -e 'issue #456' docs/contributing/build-conventions.md; then
  ok
else
  bad "canonical visibility policy missing (want ## Visibility with //tools/ci:visibility_guards plus tools/visibility/visibility.bzl plus issue #456 in docs/contributing/build-conventions.md)"
fi

# The contract owns every table the scan below reads; BUILD files never
# re-spell multi-entry scopes.
if grep -q -F -e 'PUBLIC_PACKAGES = [' "$VIS_BZL" &&
  grep -q -F -e 'PRIVATE_ENV_PACKAGES = [' "$VIS_BZL" &&
  grep -q -F -e 'PRIVATE_GAZELLE_PACKAGES = [' "$VIS_BZL" &&
  grep -q -F -e 'SCOPED_PACKAGES = {' "$VIS_BZL" &&
  grep -q -F -e 'EXPLICIT_PUBLIC_TARGETS = {' "$VIS_BZL" &&
  grep -q -F -e 'SCOPED_TARGET_GRANTS = [' "$VIS_BZL" &&
  grep -q -F -e 'LAYER_FORBIDDEN_DEPS = [' "$VIS_BZL"; then
  ok
else
  bad "visibility contract drifted (want every table in $VIS_BZL)"
fi

# Structural scan: package defaults, scoped loads, explicit publics,
# buildifier-canonical visibility stanzas, and layering edges. Hermetic
# stdlib python3 over checked-in files only (no build, no network).
scan_out="$(python3 - "$VIS_BZL" <<'PYEOF'
import ast
import re
import subprocess
import sys

contract_path = sys.argv[1]
tree = ast.parse(open(contract_path).read())
tables = {}
for node in tree.body:
    if type(node) is ast.Assign and len(node.targets) == 1 and type(node.targets[0]) is ast.Name:
        try:
            tables[node.targets[0].id] = ast.literal_eval(node.value)
        except Exception:
            pass

def need(name):
    if name not in tables:
        print("FAIL contract: table %s missing or not a literal in %s" % (name, contract_path))
        sys.exit(0)
    return tables[name]

public = need("PUBLIC_PACKAGES")
private_env = need("PRIVATE_ENV_PACKAGES")
private_gazelle = need("PRIVATE_GAZELLE_PACKAGES")
facade = need("DX_FACADE_PACKAGE")
scoped = need("SCOPED_PACKAGES")
explicit_targets = need("EXPLICIT_PUBLIC_TARGETS")
explicit_exports = need("EXPLICIT_PUBLIC_EXPORTS")
target_grants = need("SCOPED_TARGET_GRANTS")
layering = need("LAYER_FORBIDDEN_DEPS")
scope_values = {}
for node in tree.body:
    if type(node) is ast.Assign and len(node.targets) == 1 and type(node.targets[0]) is ast.Name:
        name = node.targets[0].id
        if name.startswith("VIS_"):
            try:
                scope_values[name] = ast.literal_eval(node.value)
            except Exception:
                pass

files = subprocess.run(
    ["git", "ls-files", "*.bazel"], capture_output=True, text=True, check=True
).stdout.split()
build_files = sorted(f for f in files if f.endswith("BUILD.bazel"))
texts = {f: open(f).read() for f in build_files}

legs = []
def leg(name, failures):
    legs.append((name, failures))

# Package defaults: Name (contract load) or singleton literal. The
# default_visibility may sit beside other package() attrs (notably
# default_testonly on fixtures), so match the attribute anywhere rather
# than a single-line package() spelling. Files without package() carry
# Bazel's implicit private default and stay exempt here.
VIS_LOAD = 'load("//tools/visibility:visibility.bzl"'
pkg_default = {}
default_fail = []
name_re = re.compile(r"default_visibility\s*=\s*([A-Z_][A-Z0-9_]*)\b")
lit_re = re.compile(r"default_visibility\s*=\s*(\[.*?\])", re.S)
for f, text in texts.items():
    m = name_re.search(text)
    if m:
        pkg_default[f] = ("name", m.group(1))
        continue
    m = lit_re.search(text)
    if m:
        try:
            pkg_default[f] = ("lit", ast.literal_eval(m.group(1)))
        except Exception:
            default_fail.append("%s: unparsable package default" % f)
        continue
    if "package(" in text:
        default_fail.append("%s: package() without a readable default_visibility" % f)
leg("package-defaults-readable", default_fail)

# Public defaults match the contract exactly.
want_public = sorted(p + "/BUILD.bazel" for p in public)
got_public = sorted(f for f, v in pkg_default.items() if v == ("lit", ["//visibility:public"]))
if got_public == want_public:
    leg("public-defaults", [])
else:
    leg("public-defaults", ["want %s got %s" % (" ".join(want_public), " ".join(got_public))])

# Private defaults match the contract exactly (env plans, gazelle infra, facade).
want_private = sorted(p + "/BUILD.bazel" for p in (private_env + private_gazelle + [facade]))
got_private = sorted(f for f, v in pkg_default.items() if v == ("lit", ["//visibility:private"]))
if got_private == want_private:
    leg("private-defaults", [])
else:
    leg("private-defaults", ["want %s got %s" % (" ".join(want_private), " ".join(got_private))])

# Scoped packages load and use their contracted constant; nothing else
# references a scope constant.
scoped_fail = []
vis_users = sorted(f for f in build_files if "VIS_" in texts[f] and f != "tools/visibility/BUILD.bazel")
for pkg, const in sorted(scoped.items()):
    f = pkg + "/BUILD.bazel"
    if f not in texts:
        scoped_fail.append("%s: missing BUILD.bazel" % f)
        continue
    text = texts[f]
    if VIS_LOAD + ', "%s")' % const not in text:
        scoped_fail.append("%s: missing load of %s" % (f, const))
    if "package(default_visibility = %s)" % const not in text:
        scoped_fail.append("%s: default must be %s" % (f, const))
want_users = sorted(p + "/BUILD.bazel" for p in scoped)
if vis_users != want_users:
    scoped_fail.append("scope-constant users drifted (want %s got %s)" % (" ".join(want_users), " ".join(vis_users)))
leg("scoped-loads", scoped_fail)

# No ad-hoc multi-entry literals and no unknown scope names.
adhoc_fail = []
for f, v in sorted(pkg_default.items()):
    kind, val = v
    if kind == "name":
        pkg = f[: -len("/BUILD.bazel")]
        if scoped.get(pkg) != val:
            adhoc_fail.append("%s: uncontracted scope name %s" % (f, val))
    elif val not in (["//visibility:public"], ["//visibility:private"], ["//:__subpackages__"]):
        adhoc_fail.append("%s: ad-hoc default_visibility literal %s" % (f, val))
leg("no-adhoc-literals", adhoc_fail)

# Explicit public targets: per-file rule names with a public visibility,
# plus public exports_files, match the contract.
exp_fail = []
pub_lit = 'visibility = ["//visibility:public"]'
rule_re = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\($")
name_re2 = re.compile(r'^\s*name = "([^"]+)"')
got_files = set()
for f, text in texts.items():
    names = set()
    exports_public = False
    kind = None
    depth = 0
    rule_name = None
    rule_public = False
    for line in text.splitlines():
        stripped = line.strip()
        m = rule_re.match(stripped)
        if m and depth == 0:
            kind = m.group(1)
            rule_name = None
            rule_public = False
        depth += line.count("(") - line.count(")")
        m2 = name_re2.match(line)
        if m2 and kind is not None and rule_name is None:
            rule_name = m2.group(1)
        if pub_lit in line and kind is not None:
            if kind == "exports_files":
                exports_public = True
            else:
                rule_public = True
        if depth <= 0 and kind is not None:
            if rule_public and rule_name is not None:
                names.add(rule_name)
            kind = None
            depth = 0
    if names or exports_public:
        got_files.add(f)
    want_names = set(explicit_targets.get(f[: -len("/BUILD.bazel")], []))
    if names != want_names and (names or want_names):
        exp_fail.append("%s: explicit public targets want %s got %s" % (f, sorted(want_names), sorted(names)))
    if exports_public and f[: -len("/BUILD.bazel")] not in explicit_exports:
        exp_fail.append("%s: uncontracted public exports_files" % f)
if got_files != set(p + "/BUILD.bazel" for p in explicit_targets):
    exp_fail.append("explicit public files want %s got %s" % (
        sorted(p + "/BUILD.bazel" for p in explicit_targets), sorted(got_files)))
leg("explicit-public", exp_fail)

# Target-level multi-entry grants match the contract table.
grant_fail = []
vis_re = re.compile(r"(?<!default_)visibility = (\[.*?\])", re.S)
for f, text in texts.items():
    pkg = f[: -len("/BUILD.bazel")]
    for m in vis_re.finditer(text):
        try:
            lit = ast.literal_eval(m.group(1))
        except Exception:
            grant_fail.append("%s: unparsable visibility literal" % f)
            continue
        if len(lit) <= 1 or lit == ["//visibility:public"]:
            continue
        hit = [g for g in target_grants if g[0] == pkg and g[2] in lit]
        if not hit:
            grant_fail.append("%s: uncontracted multi-entry visibility %s" % (f, lit))
            continue
        rest = [e for e in lit if e != hit[0][2]]
        if rest not in scope_values.values():
            grant_fail.append("%s: grant base not a contracted scope %s" % (f, lit))
leg("target-grants", grant_fail)

# Buildifier leg: visibility literals stay sorted and double-quoted; the
# contracted loads sit inside the // group (after // loads, before : loads).
bfail = []
for f, text in texts.items():
    for m in re.finditer(r"visibility = (\[.*?\])", text, re.S):
        lit_text = m.group(1)
        if "'" in lit_text:
            bfail.append("%s: single-quoted visibility entry" % f)
        try:
            lit = ast.literal_eval(lit_text)
        except Exception:
            bfail.append("%s: unparsable visibility literal" % f)
            continue
        if list(lit) != sorted(lit):
            bfail.append("%s: unsorted visibility list %s" % (f, list(lit)))
    for m in re.finditer(r"package\(default_visibility = ([A-Z_][A-Z0-9_]*)\)", text):
        const = m.group(1)
        load_line = VIS_LOAD + ', "%s")' % const
        lines = text.splitlines()
        try:
            idx = lines.index(load_line)
        except ValueError:
            continue
        after_slash = [i for i, l in enumerate(lines) if l.startswith('load("//')]
        before_local = [i for i, l in enumerate(lines) if l.startswith('load(":')]
        if after_slash and idx < max(after_slash):
            bfail.append("%s: %s load sorts before a // load" % (f, const))
        if before_local and idx > min(before_local):
            bfail.append("%s: %s load sorts after a : load" % (f, const))
leg("buildifier", bfail)

# Depcheck leg: no declared BUILD edge crosses the layering.
layer_fail = []
label_re = re.compile(r'"((?://|@)[^"]*)"')
for consumer_prefix, label_prefix, exempts in layering:
    for f, text in texts.items():
        if not f.startswith(consumer_prefix):
            continue
        for line in text.splitlines():
            code = line.split(" #", 1)[0]
            if code.lstrip().startswith("#"):
                continue
            if code.lstrip().startswith("load("):
                continue
            for lab in label_re.findall(code):
                if "__" in lab:
                    continue
                if lab.startswith(label_prefix) or lab == label_prefix.rstrip(":"):
                    if lab not in exempts:
                        layer_fail.append("%s: forbidden %s edge %s" % (f, label_prefix, lab))
leg("layering", sorted(set(layer_fail)))

for name, failures in legs:
    if failures:
        for detail in failures:
            print("FAIL %s: %s" % (name, detail))
    else:
        print("PASS %s" % name)
PYEOF
)"
while IFS= read -r line; do
  case "$line" in
    PASS*) ok "${line#PASS }" ;;
    FAIL*) bad "${line#FAIL }" ;;
  esac
done <<<"$scan_out"

dx_test_summary "visibility hardening harness"
