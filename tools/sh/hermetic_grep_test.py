"""Hermetic grep helper tests (issue #1006).

Cross-platform by design (no Linux-only constraint): proves the helper
behaves identically on Linux/macOS/Windows runners, covering the host
BSD/GNU divergences it replaces (`--include`/`--exclude-dir`, `-A`
windows, `-o` extraction, `sed` field extraction).
"""

import os
import subprocess
import sys
import tempfile
import unittest

HELPER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "hermetic_grep.py")


def run(*args):
    proc = subprocess.run(
        [sys.executable, HELPER, *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return proc


class HermeticGrepTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def write(self, rel, text):
        path = os.path.join(self.tmp.name, rel)
        os.makedirs(os.path.dirname(path) or self.tmp.name, exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(text)
        return path

    def test_contains_fixed(self):
        path = self.write("a.txt", "extra_target_triples = True\nnothing here\n")
        self.assertEqual(
            run("contains", path, "--fixed", "--", "extra_target_triples").returncode, 0
        )
        self.assertEqual(
            run("contains", path, "--fixed", "--", "missing-lit").returncode, 1
        )

    def test_contains_all_patterns(self):
        path = self.write("b.txt", "alpha\nbeta\ngamma\n")
        self.assertEqual(
            run("contains", path, "--fixed", "--", "alpha", "beta").returncode, 0
        )
        self.assertEqual(
            run("contains", path, "--fixed", "--", "alpha", "nope").returncode, 1
        )

    def test_contains_re(self):
        path = self.write("c.txt", 'version = "1.26.6"\n')
        self.assertEqual(
            run("contains", path, "--re", "--", r'version = "[^"]+"').returncode, 0
        )
        self.assertEqual(
            run("contains", path, "--re", "--", r"no-match-\d+").returncode, 1
        )

    def test_absent(self):
        path = self.write("d.txt", "clean file\n")
        self.assertEqual(
            run("absent", path, "--fixed", "--", "forbidden").returncode, 0
        )
        self.assertEqual(run("absent", path, "--fixed", "--", "clean").returncode, 1)

    def test_missing_file_fails_closed(self):
        missing = os.path.join(self.tmp.name, "nope.txt")
        self.assertNotEqual(
            run("contains", missing, "--fixed", "--", "x").returncode, 0
        )
        self.assertNotEqual(run("absent", missing, "--fixed", "--", "x").returncode, 0)

    def test_tree_contains_include(self):
        # BSD grep lacks --include: the helper must filter by glob itself.
        self.write("keep.yml", "runs-on: windows-latest\n")
        self.write("skip.md", "runs-on: windows-latest\n")
        proc = run(
            "tree-contains",
            "--fixed",
            "--include",
            "*.yml",
            "--roots",
            self.tmp.name,
            "--",
            "runs-on: windows-latest",
        )
        self.assertEqual(proc.returncode, 0)
        proc = run(
            "tree-contains",
            "--fixed",
            "--include",
            "*.txt",
            "--roots",
            self.tmp.name,
            "--",
            "runs-on: windows-latest",
        )
        self.assertEqual(proc.returncode, 1)

    def test_tree_absent_with_exclude_and_allow(self):
        self.write("self.sh", "Installed Build Tools\n")
        self.write("other.sh", "Installed Build Tools never approved\n")
        # Self-exclusion plus allow-string excuses every hit.
        proc = run(
            "tree-absent",
            "--fixed",
            "--exclude",
            "self.sh",
            "--allow",
            "never approved",
            "--roots",
            self.tmp.name,
            "--",
            "Installed Build Tools",
        )
        self.assertEqual(proc.returncode, 0)
        # Without the allow, the remaining hit fails the absent check.
        proc = run(
            "tree-absent",
            "--fixed",
            "--exclude",
            "self.sh",
            "--roots",
            self.tmp.name,
            "--",
            "Installed Build Tools",
        )
        self.assertEqual(proc.returncode, 1)

    def test_tree_skips_vcs_and_bazel_dirs(self):
        self.write(os.path.join(".git", "hidden.txt"), "needle\n")
        self.write(os.path.join("bazel-out", "gen.txt"), "needle\n")
        self.write("visible.txt", "hay\n")
        proc = run("tree-contains", "--fixed", "--roots", self.tmp.name, "--", "needle")
        self.assertEqual(proc.returncode, 1)

    def test_tree_list(self):
        self.write(os.path.join("gazelle", "a", "BUILD.bazel"), "gazelle_binary\n")
        self.write(os.path.join("gazelle", "b.txt"), "gazelle_binary\n")
        proc = run(
            "tree-list",
            "--fixed",
            "--include",
            "BUILD.bazel",
            "--roots",
            self.tmp.name,
            "--",
            "gazelle_binary",
        )
        self.assertEqual(proc.returncode, 0)
        self.assertIn(os.path.join("gazelle", "a", "BUILD.bazel"), proc.stdout)
        self.assertNotIn("b.txt", proc.stdout)

    def test_context_contains_fixed(self):
        path = self.write("ci.yml", "build-windows-x86_64\nline1\nline2 secrets.leak\n")
        proc = run(
            "context-contains",
            path,
            "build-windows-x86_64",
            "-A",
            "30",
            "--anchor-fixed",
            "--fixed",
            "--",
            "secrets.",
        )
        self.assertEqual(proc.returncode, 0)
        proc = run(
            "context-contains",
            path,
            "build-windows-x86_64",
            "-A",
            "1",
            "--anchor-fixed",
            "--fixed",
            "--",
            "secrets.",
        )
        self.assertEqual(proc.returncode, 1)

    def test_context_anchor_re(self):
        path = self.write("ci2.yml", "  test:\n    needs: [build]\n")
        proc = run(
            "context-contains",
            path,
            r"^  test:",
            "-A",
            "3",
            "--anchor-re",
            "--fixed",
            "--",
            "needs: [build]",
        )
        self.assertEqual(proc.returncode, 0)

    def test_context_absent(self):
        path = self.write("ci3.yml", "build-macos-arm64\nline1\nclean\n")
        proc = run(
            "context-absent",
            path,
            "build-macos-arm64",
            "-A",
            "20",
            "--anchor-fixed",
            "--re",
            "--",
            r"secrets\.|GH_TOKEN",
        )
        self.assertEqual(proc.returncode, 0)

    def test_extract_quoted(self):
        path = self.write(
            "codegen.bzl", 'DX_CODEGEN_PLAN_OUTPUT_GROUP = "dx_codegen_plans"\n'
        )
        proc = run("extract-quoted", path, "DX_CODEGEN_PLAN_OUTPUT_GROUP = ")
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(proc.stdout.strip(), "dx_codegen_plans")

    def test_extract_re(self):
        path = self.write("pins.bzl", 'GO_SDK_VERSION = "1.26.6"\n')
        proc = run("extract-re", path, r'"[^"]+"$')
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(proc.stdout.strip(), '"1.26.6"')

    def test_extract_missing(self):
        path = self.write("empty.txt", "nothing\n")
        self.assertEqual(run("extract-quoted", path, "MISSING").returncode, 1)
        self.assertEqual(run("extract-re", path, r"\d+").returncode, 1)

    def test_tree_count(self):
        self.write("one.yml", "pin\npin\n")
        self.write("two.yml", "pin\n")
        self.write("skip.md", "pin\n")
        proc = run(
            "tree-count",
            "--fixed",
            "--include",
            "*.yml",
            "--roots",
            self.tmp.name,
            "--",
            "pin",
        )
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(proc.stdout.strip(), "3")


if __name__ == "__main__":
    unittest.main()
