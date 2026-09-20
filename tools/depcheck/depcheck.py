#!/usr/bin/env python3
"""Hermetic lockfile-consistency and declared-dependency usage checker.

(required core plus admitted foundations; remaining opens):
separate non-mutating Bazel-owned checks per language.
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

Ecosystems (admitted,):
  go     - go.mod + go.sum, categories: require (prod) plus
           `// depcheck:test` marker (dev) for fixture scope;
           `// depcheck:optional` / `// depcheck:platform` markers.
           Native lock authority is go.sum verification.
  java   - jvm_deps.toml + maven_install.json (rules_jvm_external
           lock authority with fail_if_repin_required), scopes:
           compile (prod), test (dev), plus optional/platform flags.
           Covers Java sources.
  kotlin - same lock authority as java, Kotlin sources.
  scala  - same lock authority as java, Scala sources.
  csharp - paket.dependencies + paket.lock (Paket lock authority
           via paket2bazel), groups: Main (prod), Test (dev),
           plus `// optional` / `// platform` markers.
  fsharp - same lock authority as csharp, F# sources.
  cc     - cc_deps.toml + cc_lock.json (http_archive sha256
           authority: every archive carries sha256/integrity),
           scopes: prod/test plus optional/platform flags.
           Covers C/C++ sources.

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


def normalize_go(name):
    return name.lower()


def normalize_jvm(name):
    return name.lower()


def normalize_dotnet(name):
    return name.lower()


def normalize_cc(name):
    return name.lower().replace("-", "_")


def satisfies(spec, locked):
    """Minimal semver compatibility for fixtures (no registry query).

    Strips ^ ~ = == >= <= > < and compares. Bare/^ means same major;
    ~ means same major.minor; exact (==/=) means full equality.
    Newer compatible releases in a registry never affect the result:
    only manifest spec vs locked version is compared.
    Leading `v` (Go/Maven tags) is ignored on both sides.
    """
    s = spec.strip().strip("\"'").strip()
    # Remove extras/markers after ; or [ (python) and whitespace.
    s = s.split(";")[0].strip()
    # Strip leading v for Go-style tags (v1.2.3 == 1.2.3).
    if s.startswith("v") and len(s) > 1 and s[1].isdigit():
        s = s[1:]
    lv_raw = locked.strip()
    if lv_raw.startswith("v") and len(lv_raw) > 1 and lv_raw[1].isdigit():
        locked = lv_raw[1:]
    else:
        locked = lv_raw
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
    for _tkey, tval in (data.get("target", {}) or {}).items():
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
    for _grp, items in (proj.get("optional-dependencies", {}) or {}).items():
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
    for _grp, items in (data.get("dependency-groups", {}) or {}).items():
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


def parse_go_manifest(path):
    """Parse go.mod requires (native, hermetic fixture subset).

    Understands `require <mod> <ver>` single lines and `require (...)`
    blocks. Fixture scope markers in trailing comments:
      `// depcheck:test` -> dev category (test-only),
      `// depcheck:optional` -> optional flag,
      `// depcheck:platform` -> platform flag.
    Without markers every require is prod, non-optional, non-platform.
    """
    try:
        text = Path(path).read_text(encoding="utf-8")
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    in_require = False
    for rawline in text.splitlines():
        line = rawline.strip()
        if not line or line.startswith("//") and not line.startswith("// depcheck"):
            # Skip comments except depcheck markers handled below.
            if line.startswith("module ") or line.startswith("go "):
                continue
            if line.startswith("//"):
                continue
        if line.startswith("require ("):
            in_require = True
            continue
        if in_require and line == ")":
            in_require = False
            continue
        m = None
        if line.startswith("require ") and not in_require:
            # Single-line require.
            m = re.match(r"^require\s+(\S+)\s+(\S+)(.*)$", line)
        elif in_require:
            m = re.match(r"^(\S+)\s+(\S+)(.*)$", line)
        if not m:
            continue
        mod, ver, rest = m.group(1), m.group(2), m.group(3) or ""
        # Skip indirect-only markers? No: indirect is still a declaration.
        # Strip `// indirect` but keep depcheck markers.
        category = "prod"
        optional = False
        platform = False
        low = rest.lower()
        if "depcheck:test" in low:
            category = "dev"
        if "depcheck:optional" in low or "optional" in low and "depcheck" in low:
            optional = True
        if "depcheck:platform" in low:
            platform = True
        # Handle `// optional` / `// platform` shorthand in fixtures.
        if re.search(r"//\s*optional\b", rest, re.I):
            optional = True
        if re.search(r"//\s*platform\b", rest, re.I):
            platform = True
        if re.search(r"//\s*test\b", rest, re.I):
            category = "dev"
        deps[normalize_go(mod)] = {"spec": ver, "category": category,
                                   "optional": optional, "platform": platform,
                                   "raw": mod}
    return deps, ""


def parse_go_lock(path):
    """Parse go.sum (native): `<module> <version> <hash>` lines."""
    try:
        text = Path(path).read_text(encoding="utf-8")
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) < 2:
            continue
        mod, ver = parts[0], parts[1]
        # go.sum has `<mod> <ver>/go.mod <hash>` for module graphs;
        # strip the trailing /go.mod for the version key.
        if ver.endswith("/go.mod"):
            ver = ver[:-len("/go.mod")]
        key = normalize_go(mod)
        if key not in pkgs:
            pkgs[key] = ver
    # (Fixtures are minimal; first occurrence wins.)
    if not pkgs:
        return {}, ""
    return pkgs, ""


def parse_jvm_manifest(path):
    """Parse jvm_deps.toml fixture manifest (public depcheck test API).

    Format:
      [[dep]]
      group = "junit"
      artifact = "junit"
      version = "4.13.2"
      scope = "compile"  # compile=prod, test=dev
      optional = false
      platform = false
    Key is `group:artifact` (lowercased). Native lock authority is
    maven_install.json; this TOML is the focused fixture declaration.
    """
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    for item in data.get("dep", []) or []:
        grp = str(item.get("group", "")).strip()
        art = str(item.get("artifact", "")).strip()
        ver = str(item.get("version", "*")).strip()
        if not grp or not art:
            return None, "jvm dep entry without group/artifact"
        scope = str(item.get("scope", "compile")).strip().lower()
        category = "dev" if scope in ("test", "dev") else "prod"
        key = normalize_jvm(f"{grp}:{art}")
        deps[key] = {"spec": ver, "category": category,
                     "optional": bool(item.get("optional", False)),
                     "platform": bool(item.get("platform", False)),
                     "raw": f"{grp}:{art}"}
    return deps, ""


def parse_jvm_lock(path):
    """Parse maven_install.json (native rules_jvm_external lock)."""
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    arts = data.get("artifacts", {}) or {}
    for coord, info in arts.items():
        ver = str((info or {}).get("version", ""))
        pkgs[normalize_jvm(coord)] = ver
    return pkgs, ""


def parse_dotnet_manifest(path):
    """Parse paket.dependencies (native Paket, hermetic subset).

    Understands `source`, `framework:` (ignored), `group <Name>`
    (Main=prod, Test/dev groups=dev), and `nuget <Name> <version>`
    with optional trailing `// optional` / `// platform` markers.
    """
    try:
        text = Path(path).read_text(encoding="utf-8")
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    group = "Main"
    for rawline in text.splitlines():
        line = rawline.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("//"):
            continue
        if line.lower().startswith("source ") or line.lower().startswith("framework:"):
            continue
        m = re.match(r"^group\s+(\S+)", line, re.I)
        if m:
            group = m.group(1)
            continue
        m = re.match(r"^nuget\s+(\S+)\s+(\S+)(.*)$", line, re.I)
        if not m:
            continue
        name, ver, rest = m.group(1), m.group(2), m.group(3) or ""
        category = "prod" if group.lower() in ("main", "prod", "compile") else "dev"
        optional = bool(re.search(r"//\s*optional\b", rest, re.I))
        platform = bool(re.search(r"//\s*platform\b", rest, re.I))
        deps[normalize_dotnet(name)] = {"spec": ver, "category": category,
                                        "optional": optional, "platform": platform,
                                        "raw": name}
    return deps, ""


def parse_dotnet_lock(path):
    """Parse paket.lock (native): `Name (version)` under NUGET."""
    try:
        text = Path(path).read_text(encoding="utf-8")
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    for line in text.splitlines():
        m = re.match(r"^\s*([A-Za-z0-9_.\-]+)\s+\(([^)]+)\)", line)
        if m:
            pkgs[normalize_dotnet(m.group(1))] = m.group(2).strip()
    return pkgs, ""


def parse_cc_manifest(path):
    """Parse cc_deps.toml fixture manifest (http_archive sha256 authority).

    Format:
      [[dep]]
      name = "fmt"
      version = "1.0.0"
      sha256 = "fixture-sha256-..."
      scope = "prod"  # prod or test/dev
      optional = false
      platform = false
    Every entry must carry sha256 (mirrors the every-http_archive-has-hash
    rule); missing sha256 is a consistency failure.
    """
    try:
        import tomllib
    except ImportError:
        return None, "tomllib unavailable"
    try:
        data = tomllib.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable manifest: {e}"
    deps = {}
    for item in data.get("dep", []) or []:
        name = str(item.get("name", "")).strip()
        ver = str(item.get("version", "")).strip()
        sha = str(item.get("sha256", "")).strip()
        if not name:
            return None, "cc dep entry without name"
        scope = str(item.get("scope", "prod")).strip().lower()
        category = "dev" if scope in ("test", "dev") else "prod"
        deps[normalize_cc(name)] = {"spec": ver or "*", "category": category,
                                    "optional": bool(item.get("optional", False)),
                                    "platform": bool(item.get("platform", False)),
                                    "sha256": sha, "raw": name}
    return deps, ""


def parse_cc_lock(path):
    """Parse cc_lock.json: `{"packages": {name: {version, sha256}}}`."""
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unreadable lock: {e}"
    pkgs = {}
    sha = {}
    for name, info in (data.get("packages", {}) or {}).items():
        ver = str((info or {}).get("version", ""))
        pkgs[normalize_cc(name)] = ver
        sha[normalize_cc(name)] = str((info or {}).get("sha256", ""))
    # Attach sha map via a side channel: caller re-reads for hash check.
    # Store in a global for consistency (small fixtures, no threads).
    parse_cc_lock._sha = sha
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
    if ecosystem == "go":
        n = Path(s).name
        return n.endswith("_test.go") or "/tests/" in s or "/test/" in s
    if ecosystem in ("java", "kotlin", "scala"):
        n = Path(s).name
        return ("Test" in n) or "/test/" in s.lower() or "/tests/" in s.lower()
    if ecosystem in ("csharp", "fsharp"):
        n = Path(s).name
        return ("Test" in n) or "/test/" in s.lower() or "/tests/" in s.lower()
    if ecosystem == "cc":
        n = Path(s).name.lower()
        return ("test" in n) or "/test/" in s.lower() or "/tests/" in s.lower()
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
    # Read all text once (small fixtures). Skip manifests, locks, and
    # exception files by name: usage is assessed in sources, not in
    # declarations (prevents self-matching on quoted dep names).
    skip_names = {"Cargo.toml", "Cargo.lock", "pyproject.toml", "uv.lock",
                  "package.json", "pnpm-lock.yaml",
                  "go.mod", "go.sum", "jvm_deps.toml", "maven_install.json",
                  "paket.dependencies", "paket.lock",
                  "cc_deps.toml", "cc_lock.json",
                  "depcheck_exceptions.toml"}
    texts = {}
    for f in files:
        try:
            if f.name in skip_names:
                continue
            if f.suffix in (".rs", ".py", ".js", ".ts", ".mjs", ".cjs", ".jsx", ".tsx",
                            ".go", ".java", ".kt", ".kts", ".scala",
                            ".cs", ".fs", ".fsi", ".fsx",
                            ".cc", ".cpp", ".cxx", ".c", ".h", ".hpp"):
                texts[str(f)] = f.read_text(encoding="utf-8", errors="ignore")
            else:
                # Manifests/locks/exceptions (.toml/.json/.yaml) are not
                # sources; skip binaries and other files.
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
        elif ecosystem == "go":
            # Go imports are full module paths in quoted import specs.
            # Require import context so go.mod/exception TOML quotes do
            # not self-match (those files are also skipped by name).
            pats = [
                re.compile(rf'import\s+(?:\(\s*)?["\']{re.escape(dep)}["\']'),
                re.compile(rf'["\']{re.escape(dep)}(?:/[^"\']*)?["\']'),
            ]
        elif ecosystem in ("java", "kotlin", "scala"):
            # JVM imports contain the artifact id as a substring in
            # fixtures (e.g. artifact `junit` via `import org.junit...`).
            # Also match group tail (e.g. `guava` via `com.google.guava`).
            art = dep.split(":")[-1] if ":" in dep else dep
            art_dash = art.replace("-", "_").replace(".", "_")
            pats = [
                re.compile(rf"^\s*import\s+.*{re.escape(art)}\b", re.M),
                re.compile(rf"^\s*import\s+.*{re.escape(art_dash)}\b", re.M),
            ]
        elif ecosystem in ("csharp", "fsharp"):
            # `using Foo.Bar;` (C#) / `open Foo.Bar` (F#); fixtures use
            # the package name as a substring (e.g. Newtonsoft.Json).
            base = dep.split(".")[-1] if "." in dep else dep
            pats = [
                re.compile(rf"^\s*(using|open)\s+.*{re.escape(dep)}\b", re.M | re.I),
                re.compile(rf"^\s*(using|open)\s+.*{re.escape(base)}\b", re.M | re.I),
            ]
        elif ecosystem == "cc":
            # `#include <fmt/core.h>` / `#include "fmt/x.h"`; match dep
            # name as substring (fixtures use the dep name in the path).
            cname = dep.replace("-", "_")
            pats = [
                re.compile(rf"#\s*include\s+[<\"].*{re.escape(dep)}.*[>\"]"),
                re.compile(rf"#\s*include\s+[<\"].*{re.escape(cname)}.*[>\"]"),
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
    elif args.ecosystem == "go":
        deps, err = parse_go_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_go_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    elif args.ecosystem in ("java", "kotlin", "scala"):
        deps, err = parse_jvm_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_jvm_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    elif args.ecosystem in ("csharp", "fsharp"):
        deps, err = parse_dotnet_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_dotnet_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    elif args.ecosystem == "cc":
        deps, err = parse_cc_manifest(manifest)
        if deps is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
        pkgs, err = parse_cc_lock(lock)
        if pkgs is None:
            print(f"depcheck: ERROR: {err}", file=sys.stderr)
            return 2
    else:
        print(f"depcheck: ERROR: unknown ecosystem: {args.ecosystem}", file=sys.stderr)
        return 2
    failures = []
    # CC hash authority: every manifest entry must carry sha256 and match lock.
    cc_lock_sha = {}
    if args.ecosystem == "cc":
        try:
            lock_data = json.loads(Path(lock).read_text(encoding="utf-8"))
            for n, info in (lock_data.get("packages", {}) or {}).items():
                cc_lock_sha[normalize_cc(n)] = str((info or {}).get("sha256", ""))
        except Exception:
            pass
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
        elif args.ecosystem == "cc":
            want_sha = (info.get("sha256") or "").strip()
            got_sha = (cc_lock_sha.get(name) or "").strip()
            if not want_sha:
                failures.append(f"missing sha256 for '{info.get('raw', name)}' (every http_archive carries sha256/integrity)")
            elif want_sha != got_sha:
                failures.append(
                    f"stale sha256 for '{info.get('raw', name)}': manifest has {want_sha}, lock has {got_sha}")
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
    elif args.ecosystem == "go":
        deps, err = parse_go_manifest(manifest)
    elif args.ecosystem in ("java", "kotlin", "scala"):
        deps, err = parse_jvm_manifest(manifest)
    elif args.ecosystem in ("csharp", "fsharp"):
        deps, err = parse_dotnet_manifest(manifest)
    elif args.ecosystem == "cc":
        deps, err = parse_cc_manifest(manifest)
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
    for _k, v in exc.items():
        if args.ecosystem == "python":
            nk = normalize_py(v["raw"])
        elif args.ecosystem == "rust":
            nk = v["raw"].lower()
        elif args.ecosystem == "go":
            nk = normalize_go(v["raw"])
        elif args.ecosystem in ("java", "kotlin", "scala"):
            nk = normalize_jvm(v["raw"])
        elif args.ecosystem in ("csharp", "fsharp"):
            nk = normalize_dotnet(v["raw"])
        elif args.ecosystem == "cc":
            nk = normalize_cc(v["raw"])
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
    ecosystems = ["rust", "python", "js", "ts",
                  "go", "java", "kotlin", "scala",
                  "csharp", "fsharp", "cc"]
    c = sub.add_parser("consistency", help="verify manifest vs lock (offline, non-mutating)")
    c.add_argument("--ecosystem", required=True, choices=ecosystems)
    c.add_argument("--manifest", required=True)
    c.add_argument("--lock", required=True)
    u = sub.add_parser("usage", help="verify declared deps are used in owning scope")
    u.add_argument("--ecosystem", required=True, choices=ecosystems)
    u.add_argument("--manifest", required=True)
    u.add_argument("--sources", required=True)
    u.add_argument("--exceptions", default=None)
    args = ap.parse_args(argv)
    if args.cmd == "consistency":
        return cmd_consistency(args)
    return cmd_usage(args)


if __name__ == "__main__":
    sys.exit(main())
