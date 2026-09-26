"""Hermetic grep/sed-extract replacement for qualification harnesses.

Pure-stdlib python3 with identical behavior on Linux/macOS/Windows: no host
`grep`/`sed` variance (BSD `--include`/`--exclude-dir` gaps, `-A` group
separators, `-o` quirks, BRE/ERE drift). Qualification drivers call it via
`tools/sh/lib.sh` (`dx_grep_*`, `dx_tree_*`, `dx_context_*`, `dx_extract_*`)
so `bazel run //tools/ci:*_qualification` never branches on host grep.
"""

import argparse
import fnmatch
import os
import re
import sys

SKIP_DIRS = ("bazel-*", ".git")

def _read_lines(path):
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        return handle.read().splitlines()

def _compile(patterns, fixed):
    if fixed:
        return [(p, None) for p in patterns]
    return [(p, re.compile(p)) for p in patterns]

def _line_matches(line, compiled, fixed):
    for literal, rx in compiled:
        if fixed:
            if literal in line:
                return True
        elif rx.search(line):
            return True
    return False

def _all_present(lines, patterns, fixed):
    compiled = _compile(patterns, fixed)
    for literal, rx in compiled:
        found = False
        for line in lines:
            if fixed:
                if literal in line:
                    found = True
                    break
            elif rx.search(line):
                found = True
                break
        if not found:
            return False
    return True

def cmd_contains(args):
    try:
        lines = _read_lines(args.file)
    except OSError as exc:
        print("hermetic_grep: cannot read %s: %s" % (args.file, exc), file=sys.stderr)
        return 2
    ok = _all_present(lines, args.pattern, args.fixed)
    return 0 if ok else 1

def cmd_absent(args):
    try:
        lines = _read_lines(args.file)
    except OSError as exc:
        print("hermetic_grep: cannot read %s: %s" % (args.file, exc), file=sys.stderr)
        return 2
    compiled = _compile(args.pattern, args.fixed)
    for line in lines:
        if _line_matches(line, compiled, args.fixed):
            return 1
    return 0

def _skip_dir(name, extra):
    for pat in SKIP_DIRS:
        if fnmatch.fnmatch(name, pat):
            return True
    return name in extra

def _include_ok(basename, includes):
    if not includes:
        return True
    return any(fnmatch.fnmatch(basename, pat) for pat in includes)

def _iter_tree_files(roots, includes, excludes, exclude_dirs):
    for root in roots:
        if os.path.isfile(root):
            base = os.path.basename(root)
            if base in excludes:
                continue
            if _include_ok(base, includes):
                yield root
            continue
        for base, dirs, files in os.walk(root, followlinks=False):
            dirs[:] = [d for d in dirs if not _skip_dir(d, exclude_dirs)]
            dirs.sort()
            for name in sorted(files):
                if name in excludes:
                    continue
                if not _include_ok(name, includes):
                    continue
                yield os.path.join(base, name)

def _tree_scan(args):
    compiled = _compile(args.pattern, args.fixed)
    allows = args.allow or []
    allow_paths = args.allow_path or []
    hits = []
    for path in _iter_tree_files(
        args.roots or ["."],
        args.include or [],
        args.exclude or [],
        set(args.exclude_dir or []),
    ):
        rel = os.path.normpath(path)
        if any(sub in rel for sub in allow_paths):
            continue
        try:
            lines = _read_lines(path)
        except OSError:
            continue
        for text in lines:
            if not _line_matches(text, compiled, args.fixed):
                continue
            if any(sub in text for sub in allows):
                continue
            if any(sub in ("%s:%s" % (rel, text)) for sub in allows):
                continue
            hits.append("%s:%s" % (rel, text))
            if args.mode != "list":
                return hits
    return hits

def cmd_tree_contains(args):
    args.mode = "contains"
    return 0 if _tree_scan(args) else 1

def cmd_tree_absent(args):
    args.mode = "absent"
    return 0 if not _tree_scan(args) else 1

def cmd_tree_list(args):
    args.mode = "list"
    args.allow = []
    args.allow_path = []
    seen = []
    seen_set = set()
    for path in _iter_tree_files(
        args.roots or ["."],
        args.include or [],
        args.exclude or [],
        set(args.exclude_dir or []),
    ):
        rel = os.path.normpath(path)
        try:
            lines = _read_lines(path)
        except OSError:
            continue
        compiled = _compile(args.pattern, args.fixed)
        if any(_line_matches(t, compiled, args.fixed) for t in lines):
            if rel not in seen_set:
                seen_set.add(rel)
                seen.append(rel)
    for rel in seen:
        print(rel)
    return 0

def cmd_tree_count(args):
    compiled = _compile(args.pattern, args.fixed)
    total = 0
    for path in _iter_tree_files(
        args.roots or ["."],
        args.include or [],
        args.exclude or [],
        set(args.exclude_dir or []),
    ):
        try:
            lines = _read_lines(path)
        except OSError:
            continue
        for text in lines:
            if _line_matches(text, compiled, args.fixed):
                total += 1
    print(total)
    return 0

def _anchor_matches(line, anchor, anchor_fixed):
    if anchor_fixed:
        return anchor in line
    return re.search(anchor, line) is not None

def _context_scan(args, want_present):
    try:
        lines = _read_lines(args.file)
    except OSError as exc:
        print("hermetic_grep: cannot read %s: %s" % (args.file, exc), file=sys.stderr)
        return 2
    compiled = _compile(args.pattern, args.fixed)
    window = args.after + 1
    for idx, line in enumerate(lines):
        if not _anchor_matches(line, args.anchor, args.anchor_fixed):
            continue
        for text in lines[idx : idx + window]:
            if _line_matches(text, compiled, args.fixed):
                return 0 if want_present else 1
    return 1 if want_present else 0

def cmd_context_contains(args):
    return _context_scan(args, True)

def cmd_context_absent(args):
    return _context_scan(args, False)

def cmd_extract_quoted(args):
    try:
        lines = _read_lines(args.file)
    except OSError as exc:
        print("hermetic_grep: cannot read %s: %s" % (args.file, exc), file=sys.stderr)
        return 2
    for line in lines:
        if args.lit not in line:
            continue
        first = line.find('"')
        last = line.rfind('"')
        if first == -1 or last <= first:
            continue
        print(line[first + 1 : last])
        return 0
    return 1

def cmd_extract_re(args):
    try:
        lines = _read_lines(args.file)
    except OSError as exc:
        print("hermetic_grep: cannot read %s: %s" % (args.file, exc), file=sys.stderr)
        return 2
    rx = re.compile(args.re)
    for line in lines:
        found = rx.search(line)
        if found:
            print(found.group(0))
            return 0
    return 1

def build_parser():
    parser = argparse.ArgumentParser(
        prog="hermetic_grep", description="Hermetic grep replacement."
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    def add_match_flags(p):
        group = p.add_mutually_exclusive_group()
        group.add_argument("--fixed", dest="fixed", action="store_true", default=True)
        group.add_argument("--re", dest="fixed", action="store_false")

    contains = sub.add_parser(
        "contains", help="Exit 0 when every PATTERN matches FILE."
    )
    contains.add_argument("file")
    contains.add_argument("pattern", nargs="+")
    add_match_flags(contains)
    contains.set_defaults(func=cmd_contains)

    absent = sub.add_parser("absent", help="Exit 0 when no PATTERN matches FILE.")
    absent.add_argument("file")
    absent.add_argument("pattern", nargs="+")
    add_match_flags(absent)
    absent.set_defaults(func=cmd_absent)

    def add_tree_flags(p):
        p.add_argument("pattern", nargs="+")
        p.add_argument("--roots", nargs="*", default=[])
        p.add_argument("--include", action="append", default=[])
        p.add_argument("--exclude", action="append", default=[])
        p.add_argument("--exclude-dir", action="append", default=[])
        p.add_argument("--allow", action="append", default=[])
        p.add_argument("--allow-path", action="append", default=[])
        add_match_flags(p)

    tree_contains = sub.add_parser(
        "tree-contains", help="Exit 0 when any PATTERN matches under ROOTS."
    )
    add_tree_flags(tree_contains)
    tree_contains.set_defaults(func=cmd_tree_contains)

    tree_absent = sub.add_parser(
        "tree-absent", help="Exit 0 when no PATTERN matches under ROOTS."
    )
    add_tree_flags(tree_absent)
    tree_absent.set_defaults(func=cmd_tree_absent)

    tree_list = sub.add_parser(
        "tree-list", help="Print files with a match (like grep -rl)."
    )
    tree_list.add_argument("pattern", nargs="+")
    tree_list.add_argument("--roots", nargs="*", default=[])
    tree_list.add_argument("--include", action="append", default=[])
    tree_list.add_argument("--exclude", action="append", default=[])
    tree_list.add_argument("--exclude-dir", action="append", default=[])
    add_match_flags(tree_list)
    tree_list.set_defaults(func=cmd_tree_list)

    tree_count = sub.add_parser(
        "tree-count", help="Print the matching-line count (like grep -r ... | wc -l)."
    )
    tree_count.add_argument("pattern", nargs="+")
    tree_count.add_argument("--roots", nargs="*", default=[])
    tree_count.add_argument("--include", action="append", default=[])
    tree_count.add_argument("--exclude", action="append", default=[])
    tree_count.add_argument("--exclude-dir", action="append", default=[])
    add_match_flags(tree_count)
    tree_count.set_defaults(func=cmd_tree_count)

    def add_context_flags(p):
        p.add_argument("file")
        p.add_argument("anchor")
        p.add_argument("pattern", nargs="+")
        p.add_argument("-A", "--after", type=int, required=True)
        anchor = p.add_mutually_exclusive_group()
        anchor.add_argument(
            "--anchor-fixed", dest="anchor_fixed", action="store_true", default=False
        )
        anchor.add_argument("--anchor-re", dest="anchor_fixed", action="store_false")
        add_match_flags(p)

    context_contains = sub.add_parser(
        "context-contains",
        help="Exit 0 when PATTERN matches within -A lines after ANCHOR.",
    )
    add_context_flags(context_contains)
    context_contains.set_defaults(func=cmd_context_contains)

    context_absent = sub.add_parser(
        "context-absent",
        help="Exit 0 when PATTERN never matches within -A lines after ANCHOR.",
    )
    add_context_flags(context_absent)
    context_absent.set_defaults(func=cmd_context_absent)

    extract_quoted = sub.add_parser(
        "extract-quoted",
        help='Print first "..." value on the first line containing LIT.',
    )
    extract_quoted.add_argument("file")
    extract_quoted.add_argument("lit")
    extract_quoted.set_defaults(func=cmd_extract_quoted)

    extract_re = sub.add_parser(
        "extract-re", help="Print the first regex match (like grep -o -E | head -1)."
    )
    extract_re.add_argument("file")
    extract_re.add_argument("re")
    extract_re.set_defaults(func=cmd_extract_re)

    return parser

def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.cmd.startswith("tree-") and not getattr(args, "roots", []):
        args.roots = ["."]
    return args.func(args)

if __name__ == "__main__":
    sys.exit(main())
