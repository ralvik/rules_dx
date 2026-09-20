"""Go/JVM/dotnet/CC manifest and lock parsers. Split from `depcheck.py`. No behavior change."""

import argparse
import json
import re
import sys
from pathlib import Path
from depcheck_base import normalize_go, normalize_jvm, normalize_dotnet, normalize_cc

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


