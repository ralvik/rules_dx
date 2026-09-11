"""Vendored Bazel execution preset generator (M05 WP4, O62).

Source of truth for the repository `.bazelrc` execution policy. Renders two
checked-in generated files (do not edit by hand):

- `bazelrc-preset.bzl`: Starlark data (`PRESET_BAZEL_VERSION`, `PRESET_FLAGS`,
  `EXTRA_PRESETS`) pinned by `preset.update_test`.
- `preset.bazelrc`: `.bazelrc` fragment imported by the root `.bazelrc`.

Upstream flag recommendations enter only as reviewed inventory edits below;
the generator never fetches. The preset is version-matched to the Bazel pin:
`PRESET_BAZEL_VERSION` must equal `.bazelversion`, so a version bump without
a reviewed regen fails `preset.update_test`. Regen command (writes files):

    bazel run //tools/bazelrc:preset.update

Verification command (rejects stale generated files and owned-line
collisions with the root `.bazelrc`):

    bazel run //tools/bazelrc:preset.update -- --verify-only

A direct `python3 preset.py` invocation also works; paths resolve from the
script's own directory instead of BUILD_WORKSPACE_DIRECTORY.
"""

import difflib
import os
import sys

# Version-matched pin (M05 WP4): must equal `.bazelversion`. Bump only in a
# reviewed update-loop pass (version-bump PR, regen, flag-diff review, full
# verification); dependency-update automation proposes the bump, the regen
# and review stay manual per O53.
PRESET_BAZEL_VERSION = "9.2.0"

# Reviewed upstream-derived execution flags: (rc line, review rationale).
# Sources: M00 seed `.bazelrc` lines reviewed against the Bazel 9.2.0
# documented flags. New upstream recommendations arrive as reviewed edits
# here, never as fetched content.
UPSTREAM_FLAGS = [
    ("common --enable_bzlmod",
     "Bzlmod is the only supported dependency mechanism (MODULE.bazel); "
     "explicit so a future Bazel default flip cannot silently change "
     "resolution."),
    ("build --verbose_failures",
     "Full failure output for hermetic actions; CI diagnostics need no "
     "second log fetch."),
    ("test --test_output=errors",
     "Show output only for failing tests; keeps local and CI logs "
     "reviewable as the suite grows."),
]

# Owned extra_presets groups (M05 WP4, O62): name to list of (rc line,
# review rationale). Project-owned from the start; consumer delivery is
# M27 scope.
EXTRA_PRESETS = {
    "coverage": [
        ("coverage --test_env=GENERATE_LLVM_LCOV=1",
         "Bazel 9 only invokes the rules_rust collect_coverage step when "
         "GENERATE_LLVM_LCOV is set; without it `bazel coverage` silently "
         "emits empty coverage.dat files on GCC hosts."),
        ("coverage --combined_report=lcov",
         "Single combined lcov report for the coverage-gate consumer."),
        ("coverage --test_tag_filters=-no-coverage",
         "Exclude process-spawning golden tests whose instrumented child output "
         "would change the asserted stderr contract."),
        ("coverage --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov",
         "Work around Bazel 9 rules_go's unconditional C/C++ coverage helper, "
         "which requires the selected Linux seed host gcov path."),
        ("coverage --instrumentation_filter=^//",
         "Instrument workspace-owned targets only (`@` external repos never "
         "match `^//`). The filter matches labels, so a prefix ending in "
         "`/` would silently skip package-level targets; the gate scopes "
         "enforcement via the source inventory."),
    ],
}


def _source_dir():
    """Directory holding the preset package.

    Under `bazel run` the script lives in runfiles, so the workspace root
    comes from BUILD_WORKSPACE_DIRECTORY; a direct `python3` invocation
    uses the script's own directory.
    """
    workspace = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if workspace:
        return os.path.join(workspace, "tools", "bazelrc")
    return os.path.dirname(os.path.abspath(__file__))


def _render_bzl():
    lines = [
        '"""Vendored Bazel execution preset (M05 WP4, O62) -- GENERATED, do not edit.',
        "",
        "Version-matched to Bazel %s (`.bazelversion`). Upstream-derived flags "
        "and owned" % PRESET_BAZEL_VERSION,
        "`extra_presets` groups, each reviewed in `tools/bazelrc/preset.py`. "
        "Regenerate:",
        "",
        "    bazel run //tools/bazelrc:preset.update",
        '"""',
        "",
        'PRESET_BAZEL_VERSION = "%s"' % PRESET_BAZEL_VERSION,
        "",
        "PRESET_FLAGS = [",
    ]
    lines += ['    "%s",' % line for line, _review in UPSTREAM_FLAGS]
    lines += [
        "]",
        "",
        "EXTRA_PRESETS = {",
    ]
    for group in sorted(EXTRA_PRESETS):
        lines += ['    "%s": [' % group]
        lines += ['        "%s",' % line for line, _review in EXTRA_PRESETS[group]]
        lines += ["    ],"]
    lines += [
        "}",
        "",
    ]
    return "\n".join(lines)


def _render_fragment():
    lines = [
        "# Vendored Bazel execution preset -- GENERATED, do not edit.",
        "# Version-matched to Bazel %s (`.bazelversion`); reviewed inventory in "
        "`tools/bazelrc/preset.py`." % PRESET_BAZEL_VERSION,
        "# Regenerate: `bazel run //tools/bazelrc:preset.update`.",
        "# Upstream-derived flags (reviewed):",
    ]
    lines += [line for line, _review in UPSTREAM_FLAGS]
    for group in sorted(EXTRA_PRESETS):
        lines += ["# Owned extra_presets group: %s." % group]
        lines += [line for line, _review in EXTRA_PRESETS[group]]
    lines += [""]
    return "\n".join(lines)


def _rendered_files(source_dir):
    return {
        os.path.join(source_dir, "bazelrc-preset.bzl"): _render_bzl(),
        os.path.join(source_dir, "preset.bazelrc"): _render_fragment(),
    }


def _owned_collisions(workspace_root, rendered_lines):
    """Root `.bazelrc` lines duplicating preset-rendered flag lines.

    Project overrides stay explicit only for non-preset flags; `import`,
    `try-import`, comments, and blanks are ignored.
    """
    collisions = []
    path = os.path.join(workspace_root, ".bazelrc")
    with open(path, encoding="utf-8") as handle:
        for raw in handle.read().splitlines():
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("import ") or line.startswith("try-import "):
                continue
            if line in rendered_lines:
                collisions.append(line)
    return collisions


def main(argv):
    verify_only = argv == ["--verify-only"]
    if argv and not verify_only:
        sys.exit("usage: preset.update [--verify-only]")
    source_dir = _source_dir()
    if not os.path.isdir(source_dir):
        sys.exit("preset.update: source directory %r not found" % source_dir)
    rendered = _rendered_files(source_dir)
    rendered_lines = set()
    for content in rendered.values():
        for line in content.splitlines():
            if line and not line.startswith("#"):
                rendered_lines.add(line)
    workspace_root = os.path.dirname(os.path.dirname(source_dir))
    collisions = _owned_collisions(workspace_root, rendered_lines)
    if collisions:
        sys.exit("preset.update: root .bazelrc duplicates preset lines; "
                 "reconcile (remove owned duplicates, keep project overrides "
                 "explicit): %s" % ", ".join(sorted(collisions)))
    failed = False
    for path, content in rendered.items():
        if verify_only:
            with open(path, encoding="utf-8") as handle:
                checked_in = handle.read()
            if checked_in != content:
                print("preset.update: %s is stale; run the regen command"
                      % os.path.basename(path))
                for diff in difflib.unified_diff(
                        checked_in.splitlines(), content.splitlines(),
                        "checked-in", "regenerated", lineterm=""):
                    print(diff)
                failed = True
            else:
                print("preset.update: %s verified" % os.path.basename(path))
        else:
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(content)
            print("preset.update: wrote %s" % os.path.basename(path))
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main(sys.argv[1:])
