#!/usr/bin/env python3
"""Local output test."""

import hashlib
import os
import tempfile
import unittest

from github_deploy import build_staging, release_command


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class StagingTest(unittest.TestCase):
    def test_builds_release_dir_plus_manifest_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            first = os.path.join(tmp, "release_demo.tar.gz")
            with open(first, "wb") as f:
                f.write(b"github-asset-one-v1")
            second = os.path.join(tmp, "release_demo.tar.gz.sha256")
            with open(second, "wb") as f:
                f.write(b"github-asset-two-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            staging = build_staging(
                [first, second], outdir, "github_demo", "v0.0.0-dryrun"
            )

            self.assertEqual(staging, os.path.join(outdir, "github_demo-release"))
            for src in (first, second):
                dest = os.path.join(staging, os.path.basename(src))
                self.assertTrue(os.path.isfile(dest), dest)
                self.assertEqual(_sha256(dest), _sha256(src))
            manifest = os.path.join(staging, "would-run.txt")
            self.assertTrue(os.path.isfile(manifest), manifest)
            with open(manifest, encoding="utf-8") as f:
                body = f.read()
            self.assertIn("gh release create v0.0.0-dryrun", body)
            self.assertIn("--draft --verify-tag", body)
            self.assertIn("release_demo.tar.gz", body)
            self.assertIn(_sha256(first), body)

    def test_release_command_pins_draft_only_flags(self):
        cmd = release_command("v0.0.0-dryrun", ["a.tar.gz", "a.tar.gz.sha256"])
        self.assertEqual(
            cmd,
            "gh release create v0.0.0-dryrun a.tar.gz a.tar.gz.sha256 "
            "--draft --verify-tag",
        )

    def test_rejects_empty_assets(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(ValueError):
                build_staging([], outdir, "github_demo", "v0.0.0-dryrun")


if __name__ == "__main__":
    unittest.main()
