"""Consistency and usage commands. Split from `depcheck.py`. No behavior change."""

import argparse
import json
import re
import sys
from pathlib import Path
from depcheck_base import fail, error, satisfies, normalize_py, normalize_js, normalize_rs, normalize_go, normalize_jvm, normalize_dotnet, normalize_cc
from depcheck_parsers_core import parse_rust_manifest, parse_rust_lock, parse_python_manifest, parse_python_lock, parse_js_manifest, parse_pnpm_lock
from depcheck_parsers_extra import parse_go_manifest, parse_go_lock, parse_jvm_manifest, parse_jvm_lock, parse_dotnet_manifest, parse_dotnet_lock, parse_cc_manifest, parse_cc_lock
from depcheck_usages import parse_exceptions, iter_sources, is_test_file, find_usages

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


