"""Shared fail/error plus normalize/satisfies helpers. Split from `depcheck.py`. No behavior change."""

import argparse
import json
import re
import sys
from pathlib import Path

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

