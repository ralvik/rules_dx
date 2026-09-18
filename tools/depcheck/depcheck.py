#!/usr/bin/env python3
"""Hermetic lockfile-consistency and declared-dependency usage checker.

Issue #22: separate non-mutating Bazel-owned checks per language.
Offline, no network, no registry queries, no manifest/lock writes.
Only reads declared inputs; missing inputs fail actionably (exit 2).

Ecosystems (required core):
  rust   - Cargo.toml + Cargo.lock, categories: dependencies (prod),
           dev-dependencies (test), build-dependencies (build),
           target-specific + optional.
  python - pyproject.toml + uv.lock, categories: project.dependencies
           (prod), dependency-groups/dev + optional-dependencies (dev),
           markers (platform).
  js     - package.json + pnpm-lock.yaml, categories: dependencies
           (prod), devDependencies (dev), optionalDependencies
           (optional/platform). Covers JavaScript and TypeScript sources.

Usage is assessed across the owning scope (all sources under --sources),
not one target. Transitive locked packages not directly declared are
ignored. Platform/optional deps used in any supported config pass
without exceptions; declaration as optional/platform is not proof.
Prod used only by tests fails with a category error. Narrow
dependency-scoped exceptions with reasons cover legitimate non-import
uses; missing reasons, unrelated unused, and obsolete entries fail.
Validation reports without deleting.
"""
import argparse
import json
import re
import sys
from pathlib import Path

# No network imports by design (offline route): no urllib, socket, http,
# requests. Verified by depcheck tests via source grep.


def fail(msg):
    print(f"depcheck: FAIL: {msg}", file=sys.stderr)
    return 1


def error(msg):
    print(f"depcheck: ERROR: {msg}", file=sys.stderr)
    return 2


def normalize_py(name):
    return name.lower().replace("-", "_").replace(".", "_")


def normalize_js(name):
    return name.lower()


def normalize_rs(name):
    return name.lower().replace("-", "_")


def satisfies(spec, locked):
    """Minimal semver compatibility for fixtures (no registry query).

    Strips ^ ~ = == >= <= > < and compares. Bare/^ means same major;
    ~ means same major.minor; exact (==/=) means full equality.
    Newer compatible releases in a registry never affect the result:
    only manifest spec vs locked version is compared.
    """
    s = spec.strip().strip("\"'").strip()
    # Remove extras/markers after ; or [ (python) and whitespace.
    s = s.split(";")[0].strip()
    # For python "name>=1.2" style, caller passes version part only.
    # Handle common prefixes.
    exact = False
    if s.startswith("=="):
        s = s[2:].strip()
        exact = True
    elif s.startswith("="):
        s = s[1:].strip()
        exact = True
    elif s.startswith("^"):
        s = s[1:].strip()
    elif s.startswith("~"):
        # ~1.2.3 => same major.minor
        s = s[1:].strip()
        lv = locked.strip().split("+")[0]
        try:
            sp = [int(x) for x in re.split(r"[.\-]", s)[:2]]
            lp = [int(x) for x in re.split(r"[.\-]", lv)[:2]]
            return sp == lp
        except ValueError:
            return s == lv
    elif s.startswith(">=") or s.startswith("<="):
        # For fixtures treat >=X as satisfied if locked >= X by tuple.
        op = s[:2]
        s = s[2:].strip().split(",")[0].strip()
        try:
            def tup(v):
                return tuple(int(x) if x.isdigit() else 0 for x in re.split(r"[.\-]", v.split("+")[0])[:3])
            if op == ">=":
                return tup(locked) >= tup(s)
            return tup(locked) <= tup(s)
        except ValueError:
            return False
    # Bare versions: "*" always passes; exact X.Y.Z requires equality
    # (npm exact pins); short "1"/"1.2" use caret semantics (same major)
    # for Cargo-style fixtures. Newer registry releases never matter.
    if s in ("*", ""):
        return True
    if exact:
        return locked.strip() == s
    if re.fullmatch(r"\d+\.\d+\.\d+.*", s):
        return locked.strip() == s
    # Caret/bare: same major.
    try:
        smajor = s.split(".")[0].strip()
        lmajor = locked.strip().split(".")[0].strip()
        # If spec has only major ("1"), compare major.
        if re.fullmatch(r"\d+", smajor):
            return smajor == lmajor
        # Full spec: require major match (caret semantics for 1.x).
        return smajor == lmajor
    except IndexError:
        return False


# --- manifest parsing ---

def parse_rust_manifest(path):
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable (need python>=3.11)"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}  # name -> {spec, category, optional, platform}
    for cat, key in (("prod", "dependencies"), ("dev", "dev-dependencies"), ("build", "build-dependencies")):
        tbl = data.get(key, {}) or {}
        for name, val in tbl.items():
            spec = ""
            optional = False
            if isinstance(val, str):
                spec = val
            elif isinstance(val, dict):
                spec = str(val.get("version", "*"))
                optional = bool(val.get("optional", False))
            deps[name.lower()] = {"spec": spec, "category": cat, "optional": optional, "platform": False}
    # target-specific tables: [target.<cfg>.dependencies]
    for tkey, tval in (data.get("target", {}) or {}).items():
        if not isinstance(tval, dict):
            continue
        for sub in ("dependencies", "dev-dependencies", "build-dependencies"):
            tbl = tval.get(sub, {}) or {}
            cat = "prod" if sub == "dependencies" else ("dev" if sub.startswith("dev") else "build")
            for name, val in tbl.items():
                spec = val if isinstance(val, str) else str(val.get("version", "*"))
                deps[name.lower()] = {"spec": spec, "category": cat, "optional": False, "platform": True}
    # features do not create deps; optional deps already flagged.
    return deps, ""


def parse_rust_lock(path):
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    for p in data.get("package", []) or []:
        name = str(p.get("name", "")).lower()
        ver = str(p.get("version", ""))
        if name:
            pkgs[name] = ver
    return pkgs, ""


def parse_python_manifest(path):
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    proj = data.get("project", {}) or {}
    for item in proj.get("dependencies", []) or []:
        # "name>=1.2; sys_platform=='win32'" -> name, spec, marker
        m = re.match(r"^\s*([A-Za-z0-9_.\-]+(?:\[[^\]]*\])?)\s*(.*)$", item)
        if not m:
            continue
        rawname = m.group(1).split("[")[0]
        rest = m.group(2).strip()
        marker_platform = "sys_platform" in rest or "sys-platform" in rest or "platform_system" in rest or "os_name" in rest
        # version spec is leading [=> <>=~! ] chunk
        vm = re.match(r"^([=<>^~!0-9.\s,*]+)", rest)
        spec = vm.group(1).strip() if vm else "*"
        if not spec:
            spec = "*"
        deps[normalize_py(rawname)] = {"spec": spec, "category": "prod",
                                       "optional": False, "platform": marker_platform,
                                       "raw": rawname}
    for grp, items in (proj.get("optional-dependencies", {}) or {}).items():
        for item in items or []:
            m = re.match(r"^\s*([A-Za-z0-9_.\-]+)", item)
            if not m:
                continue
            rawname = m.group(1)
            rest = item[len(m.group(0)):].strip()
            vm = re.match(r"^([=<>^~!0-9.\s,*]+)", rest)
            spec = vm.group(1).strip() if vm else "*"
            if not spec:
                spec = "*"
            # optional-dependencies are dev-ish extras; category dev so
            # test-only use does not raise a prod category error, but
            # unused still fails (optional is not proof of usage).
            deps[normalize_py(rawname)] = {"spec": spec, "category": "dev",
                                           "optional": True, "platform": False, "raw": rawname}
    for grp, items in (data.get("dependency-groups", {}) or {}).items():
        if not isinstance(items, list):
            continue
        for item in items:
            if not isinstance(item, str):
                continue
            m = re.match(r"^\s*([A-Za-z0-9_.\-]+)", item)
            if not m:
                continue
            rawname = m.group(1)
            rest = item[len(m.group(0)):].strip()
            vm = re.match(r"^([=<>^~!0-9.\s,*]+)", rest)
            spec = vm.group(1).strip() if vm else "*"
            if not spec:
                spec = "*"
            deps[normalize_py(rawname)] = {"spec": spec, "category": "dev",
                                           "optional": False, "platform": False, "raw": rawname}
    return deps, ""


def parse_python_lock(path):
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    for p in data.get("package", []) or []:
        name = str(p.get("name", ""))
        ver = str(p.get("version", ""))
        if name:
            pkgs[normalize_py(name)] = ver
    # uv.lock may also list under different shapes; fall back to regex
    # scan for name/version pairs if empty (keeps fixtures minimal).
    if not pkgs:
        text = Path(path).read_text(encoding="utf-8")
        for m in re.finditer(r'name\s*=\s*"([^"]+)"\s*\n\s*version\s*=\s*"([^"]+)"', text):
            pkgs[normalize_py(m.group(1))] = m.group(2)
    return pkgs, ""


def parse_js_manifest(path):
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    for name, spec in (data.get("dependencies", {}) or {}).items():
        deps[normalize_js(name)] = {"spec": str(spec), "category": "prod", "optional": False, "platform": False}
    for name, spec in (data.get("devDependencies", {}) or {}).items():
        deps[normalize_js(name)] = {"spec": str(spec), "category": "dev", "optional": False, "platform": False}
    for name, spec in (data.get("optionalDependencies", {}) or {}).items():
        deps[normalize_js(name)] = {"spec": str(spec), "category": "prod", "optional": True, "platform": True}
    for name, spec in (data.get("peerDependencies", {}) or {}).items():
        # peers are not lock-owned in fixtures; record as prod but do
        # not require lock entries (handled by caller skipping peers).
        deps[normalize_js(name)] = {"spec": str(spec), "category": "prod", "optional": False, "platform": False, "peer": True}
    return deps, ""


def parse_pnpm_lock(path):
    """Minimal pnpm-lock.yaml parser for fixtures (no yaml dep).

    Understands:
      importers: (ignored for package set)
      packages:
        <name>@<version>: ...
    plus legacy 'dependencies:' lines. Returns name->version.
    """
    try:
        text = Path(path).read_text(encoding="utf-8")
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    in_packages = False
    for line in text.splitlines():
        if re.match(r"^packages\s*:", line):
            in_packages = True
            continue
        if in_packages and re.match(r"^[A-Za-z]", line) and not line.startswith(" ") and not line.startswith("\t"):
            # next top-level section ends packages (e.g. snapshots:)
            if not line.strip().startswith("'") and not line.strip().startswith('"'):
                if re.match(r"^\S+:\s*$", line):
                    in_packages = False
                    continue
        if in_packages:
            m = re.match(r"""\s*['"]?([^'"\s:]+)@([^\s:'"]+)['"]?\s*:""", line)
            if m:
                name, ver = m.group(1), m.group(2)
                # pnpm keys may be '@scope/name@version' -> split carefully
                # e.g. "'@astrojs/compiler@4.0.0':" -> name @astrojs/compiler
                raw = line.strip().strip("'\"")
                # find last @ before version
                at = raw.rfind("@")
                if at > 0:
                    name = raw[:at].strip("'\" ")
                    ver = raw[at + 1:].split(":")[0].strip("'\" ()")
                    ver = ver.split("(")[0]
                    pkgs[normalize_js(name)] = ver
                else:
                    pkgs[normalize_js(name)] = ver
    return pkgs, ""


def parse_exceptions(path):
    if path is None:
        return {}, ""
    p = Path(path)
    if not p.exists():
        return None, f"exceptions file missing: {path}"
    try:
        import tomllib
        data = tomllib.loads(p.read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable exceptions: {e}"
    out = {}
    for item in data.get("exception", []) or []:
        name = str(item.get("dependency", "")).strip()
        reason = str(item.get("reason", "")).strip()
        if not name:
            return None, "exception entry without dependency"
        # key normalized per ecosystem by caller; store raw lower here
        out[name.lower().replace("-", "_")] = {"raw": name, "reason": reason}
    return out, ""


# --- source scanning ---

def iter_sources(root):
    for p in sorted(Path(root).rglob("*")):
        if p.is_file():
            yield p


def is_test_file(ecosystem, path):
    s = str(path).replace("\\", "/")
    if ecosystem == "rust":
        return "/tests/" in s or s.endswith("_test.rs") or "test" in Path(s).name
    if ecosystem == "python":
        n = Path(s).name
        return n.startswith("test_") or n.endswith("_test.py") or "/tests/" in s
    # js/ts
    n = Path(s).name
    return n.endswith(".test.js") or n.endswith(".test.ts") or "/__tests__/" in s or "/tests/" in s


def find_usages(ecosystem, sources_root, dep_names):
    """Return {depkey: {'src': bool, 'test': bool, 'build': bool}}.

    Scans all sources in owning scope (workspace-wide). No execution,
    no foreign-platform binaries; platform files count by text.
    """
    out = {k: {"src": False, "test": False, "build": False} for k in dep_names}
    files = list(iter_sources(sources_root))
    # Read all text once (small fixtures).
    texts = {}
    for f in files:
        try:
            if f.suffix in (".rs", ".py", ".js", ".ts", ".mjs", ".cjs", ".jsx", ".tsx", ".toml", ".json", ".yaml", ".yml"):
                texts[str(f)] = f.read_text(encoding="utf-8", errors="ignore")
            else:
                # still scan build.rs etc. by extension already covered;
                # skip binaries.
                continue
        except Exception:
            continue
    for dep in dep_names:
        # Build regexes per ecosystem.
        if ecosystem == "rust":
            cname = dep.replace("-", "_")
            pats = [
                re.compile(rf"\buse\s+{re.escape(cname)}\b"),
                re.compile(rf"\bextern\s+crate\s+{re.escape(cname)}\b"),
                re.compile(rf"\b{cname}\s*::"),
            ]
        elif ecosystem == "python":
            mod = dep  # already normalized with _
            raw_dash = dep.replace("_", "-")
            pats = [
                re.compile(rf"^\s*import\s+{re.escape(mod)}\b", re.M),
                re.compile(rf"^\s*from\s+{re.escape(mod)}\b", re.M),
                re.compile(rf"^\s*import\s+{re.escape(raw_dash)}\b", re.M),
                re.compile(rf"^\s*from\s+{re.escape(raw_dash)}\b", re.M),
            ]
        else:
            pats = [
                re.compile(rf"""from\s+['"]{re.escape(dep)}['"]"""),
                re.compile(rf"""require\(\s*['"]{re.escape(dep)}['"]\s*\)"""),
                re.compile(rf"""import\(\s*['"]{re.escape(dep)}['"]\s*\)"""),
            ]
        for fstr, text in texts.items():
            hit = any(p.search(text) for p in pats)
            if not hit:
                continue
            p = Path(fstr)
            if p.name == "build.rs":
                out[dep]["build"] = True
            elif is_test_file(ecosystem, p):
                out[dep]["test"] = True
            else:
                out[dep]["src"] = True
    return out


def cmd_consistency(args):
    manifest = Path(args.manifest)
    lock = Path(args.lock)
    if not manifest.exists():
        print(f"depcheck: ERROR: manifest missing: {manifest}", file=sys.stderr)
        return 2
    if not lock.exists():
        print(f"depcheck: ERROR: lock missing: {lock} (declare lock inputs, do not skip)", file=sys.stderr)
        return 2
    if args.ecosystem == "rust":
        deps, err = parse_rust_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_rust_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    elif args.ecosystem == "python":
        deps, err = parse_python_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_python_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    elif args.ecosystem in ("js", "ts"):
        deps, err = parse_js_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_pnpm_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        # peers are not lock-owned; drop them from consistency.
        deps = {k: v for k, v in deps.items() if not v.get("peer")}
    else:
        print(f"depcheck: ERROR: unknown ecosystem: {args.ecosystem}", file=sys.stderr)
        return 2
    failures = []
    for name, info in sorted(deps.items()):
        locked = pkgs.get(name)
        # js scoped names: lock keys normalized lower; already handled.
        if locked is None:
            # Try dash/underscore variants for python/rust.
            alt = name.replace("_", "-") if "_" in name else name.replace("-", "_")
            locked = pkgs.get(alt)
        if locked is None:
            failures.append(f"missing lock entry for '{info.get('raw', name)}' (manifest requires {info['spec']})")
        elif not satisfies(info["spec"], locked):
            failures.append(
                f"stale lock entry for '{info.get('raw', name)}': manifest requires {info['spec']}, lock has {locked}")
    if failures:
        for f in failures:
            print(f"depcheck: FAIL: consistency: {f}", file=sys.stderr)
        return 1
    print(f"depcheck: OK: consistency: {len(deps)} declarations match lock ({args.ecosystem})")
    return 0


def cmd_usage(args):
    manifest = Path(args.manifest)
    sources = Path(args.sources)
    if not manifest.exists():
        print(f"depcheck: ERROR: manifest missing: {manifest}", file=sys.stderr)
        return 2
    if not sources.exists():
        print(f"depcheck: ERROR: sources missing: {sources}", file=sys.stderr)
        return 2
    if args.ecosystem == "rust":
        deps, err = parse_rust_manifest(manifest)
    elif args.ecosystem == "python":
        deps, err = parse_python_manifest(manifest)
    elif args.ecosystem in ("js", "ts"):
        deps, err = parse_js_manifest(manifest)
        if deps is not None:
            deps = {k: v for k, v in deps.items() if not v.get("peer")}
    else:
        print(f"depcheck: ERROR: unknown ecosystem: {args.ecosystem}", file=sys.stderr)
        return 2
    if deps is None:
        print(f"depcheck: ERROR: {err}", file=sys.stderr)
        return 2
    exc, err = parse_exceptions(args.exceptions)
    if exc is None:
        print(f"depcheck: ERROR: {err}", file=sys.stderr)
        return 2
    # Normalize exception keys per ecosystem.
    norm_exc = {}
    for k, v in exc.items():
        if args.ecosystem == "python":
            nk = normalize_py(v["raw"])
        elif args.ecosystem == "rust":
            nk = v["raw"].lower()
        else:
            nk = normalize_js(v["raw"])
        norm_exc[nk] = v
    usages = find_usages(args.ecosystem, sources, list(deps.keys()))
    failures = []
    # Obsolete: exception for removed dependency.
    for ek, ev in sorted(norm_exc.items()):
        if ek not in deps:
            failures.append(f"obsolete exception for removed dependency '{ev['raw']}' (report only, not deleted)")
    for name, info in sorted(deps.items()):
        u = usages.get(name, {"src": False, "test": False, "build": False})
        used_any = u["src"] or u["test"] or u["build"]
        has_exc = name in norm_exc
        reason = norm_exc[name]["reason"] if has_exc else ""
        if has_exc and not reason:
            failures.append(f"exception for '{info.get('raw', name)}' missing reason (each exception needs an explanatory reason)")
            continue
        if has_exc and used_any:
            # Exception no longer suppressing a finding (checker now
            # recognizes the usage, e.g. after an upgrade): obsolete.
            failures.append(
                f"obsolete exception for '{info.get('raw', name)}' (usage recognized; remove the exception)")
            continue
        if has_exc and not used_any:
            # Still-needed explained exception passes; does not waive
            # consistency (checked separately) nor suppress others.
            continue
        # No exception: evaluate usage across supported configs.
        if used_any:
            # Category validation where the ecosystem distinguishes.
            if info["category"] == "prod" and not u["src"] and not u["build"] and u["test"]:
                failures.append(
                    f"category error: production declaration '{info.get('raw', name)}' used only by tests (move to dev)")
                continue
            if args.ecosystem == "rust" and info["category"] == "prod" and not u["src"] and u["build"] and not u["test"]:
                failures.append(
                    f"category error: production declaration '{info.get('raw', name)}' used only by build tooling (move to build-dependencies)")
                continue
            # Platform/optional used anywhere (incl. other-platform
            # files) passes without exception; multi-category passes.
            continue
        # Unused in all sources.
        if info.get("optional") or info.get("platform"):
            failures.append(
                f"unused {'optional' if info.get('optional') else 'platform-specific'} declaration '{info.get('raw', name)}' (optional/platform is not proof of usage)")
        else:
            failures.append(f"unused declaration '{info.get('raw', name)}'")
    if failures:
        for f in failures:
            print(f"depcheck: FAIL: usage: {f}", file=sys.stderr)
        return 1
    print(f"depcheck: OK: usage: {len(deps)} declarations used in owning scope ({args.ecosystem})")
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(prog="depcheck")
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("consistency", help="verify manifest vs lock (offline, non-mutating)")
    c.add_argument("--ecosystem", required=True, choices=["rust", "python", "js", "ts"])
    c.add_argument("--manifest", required=True)
    c.add_argument("--lock", required=True)
    u = sub.add_parser("usage", help="verify declared deps are used in owning scope")
    u.add_argument("--ecosystem", required=True, choices=["rust", "python", "js", "ts"])
    u.add_argument("--manifest", required=True)
    u.add_argument("--sources", required=True)
    u.add_argument("--exceptions", default=None)
    args = ap.parse_args(argv)
    if args.cmd == "consistency":
        return cmd_consistency(args)
    return cmd_usage(args)


if __name__ == "__main__":
    sys.exit(main())
