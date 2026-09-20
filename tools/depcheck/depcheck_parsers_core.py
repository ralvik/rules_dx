"""Rust/Python/JS manifest and lock parsers. Split from `depcheck.py`. No behavior change."""

import argparse
import json
import re
import sys
from pathlib import Path
from depcheck_base import normalize_py, normalize_js, normalize_rs

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


