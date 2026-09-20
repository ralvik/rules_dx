"""Exceptions plus source usage discovery. Split from `depcheck.py`. No behavior change."""

import argparse
import json
import re
import sys
from pathlib import Path
from depcheck_base import normalize_py, normalize_js, normalize_rs, normalize_go, normalize_jvm, normalize_dotnet, normalize_cc

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


