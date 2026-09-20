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



import sys as _sys
sys_path_added = False
# When run from Bazel runfiles, siblings sit beside this file.
try:
    from pathlib import Path as _P
    _here = str(_P(__file__).resolve().parent)
    if _here not in _sys.path:
        _sys.path.insert(0, _here)
except Exception:
    pass

from depcheck_base import fail, error
from depcheck_parsers_core import parse_rust_manifest, parse_rust_lock, parse_python_manifest, parse_python_lock, parse_js_manifest, parse_pnpm_lock
from depcheck_parsers_extra import parse_go_manifest, parse_go_lock, parse_jvm_manifest, parse_jvm_lock, parse_dotnet_manifest, parse_dotnet_lock, parse_cc_manifest, parse_cc_lock
from depcheck_usages import parse_exceptions, iter_sources, is_test_file, find_usages
from depcheck_commands import cmd_consistency, cmd_usage

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
