#!/usr/bin/env python3
"""Local package-drop output test for `octopus_deploy` (no network, no sockets)."""

import hashlib
import os
import tempfile
import unittest

from octopus_deploy import build_drop


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class DropTest(unittest.TestCase):
    def test_builds_drop_dir_plus_manifest_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            package = os.path.join(tmp, "release_demo.tar.gz")
            with open(package, "wb") as f:
                f.write(b"octopus-demo-package-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            drop = build_drop(
                package,
                outdir,
                "octopus_demo",
                "octopus_demo",
                "Default",
                "0.0.0",
                ["Production"],
                "",
                "https://octopus.example.invalid",
            )

            self.assertEqual(drop, os.path.join(outdir, "octopus_demo-drop"))
            dest = os.path.join(drop, os.path.basename(package))
            self.assertTrue(os.path.isfile(dest), dest)
            self.assertEqual(_sha256(dest), _sha256(package))
            manifest = os.path.join(drop, "would-run.txt")
            self.assertTrue(os.path.isfile(manifest), manifest)
            with open(manifest, encoding="utf-8") as f:
                body = f.read()
            self.assertIn(
                "create-release --project octopus_demo --channel Default", body
            )
            self.assertIn("--deploy-to Production", body)
            self.assertIn(_sha256(package), body)

    def test_builds_manifest_without_deploy_to(self):
        with tempfile.TemporaryDirectory() as tmp:
            package = os.path.join(tmp, "release_demo.tar.gz")
            with open(package, "wb") as f:
                f.write(b"octopus-demo-package-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            drop = build_drop(
                package,
                outdir,
                "octopus_demo",
                "octopus_demo",
                "Default",
                "0.0.0",
                [],
                "",
                "https://octopus.example.invalid",
            )

            manifest = os.path.join(drop, "would-run.txt")
            with open(manifest, encoding="utf-8") as f:
                body = f.read()
            self.assertIn(
                "create-release --project octopus_demo --channel Default", body
            )
            self.assertNotIn("--deploy-to", body)

    def test_rejects_non_tarball_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            other = os.path.join(tmp, "release_demo.zip")
            with open(other, "wb") as f:
                f.write(b"not-a-tarball")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(ValueError):
                build_drop(
                    other,
                    outdir,
                    "octopus_demo",
                    "octopus_demo",
                    "Default",
                    "0.0.0",
                    [],
                    "",
                    "https://octopus.example.invalid",
                )


if __name__ == "__main__":
    unittest.main()
