#!/usr/bin/env python3
"""Local output test."""

import hashlib
import os
import tempfile
import unittest

from pypi_deploy import build_wheelhouse, minimal_upload_env


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class WheelhouseTest(unittest.TestCase):
    def test_builds_simple_index_plus_whl_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            wheel = os.path.join(tmp, "pypi_demo-0.0.0-py3-none-any.whl")
            with open(wheel, "wb") as f:
                f.write(b"pypi-demo-wheel-bytes-v1")
            sdist = os.path.join(tmp, "pypi_demo-0.0.0.tar.gz")
            with open(sdist, "wb") as f:
                f.write(b"pypi-demo-sdist-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            house = build_wheelhouse(wheel, sdist, outdir, "pypi_demo")

            self.assertEqual(house, os.path.join(outdir, "pypi_demo-wheelhouse"))
            for src in (wheel, sdist):
                dest = os.path.join(house, os.path.basename(src))
                self.assertTrue(os.path.isfile(dest), dest)
                self.assertEqual(_sha256(dest), _sha256(src))
            index = os.path.join(house, "simple-index", "pypi_demo", "index.html")
            self.assertTrue(os.path.isfile(index), index)
            with open(index, encoding="utf-8") as f:
                body = f.read()
            self.assertIn("pypi_demo-0.0.0-py3-none-any.whl", body)
            self.assertIn("pypi_demo-0.0.0.tar.gz", body)
            self.assertIn("#sha256=", body)

    def test_builds_wheel_only_without_sdist(self):
        with tempfile.TemporaryDirectory() as tmp:
            wheel = os.path.join(tmp, "pypi_demo-0.0.0-py3-none-any.whl")
            with open(wheel, "wb") as f:
                f.write(b"pypi-demo-wheel-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            house = build_wheelhouse(wheel, None, outdir, "pypi_demo")

            dest = os.path.join(house, os.path.basename(wheel))
            self.assertTrue(os.path.isfile(dest), dest)
            self.assertEqual(_sha256(dest), _sha256(wheel))
            index = os.path.join(house, "simple-index", "pypi_demo", "index.html")
            with open(index, encoding="utf-8") as f:
                body = f.read()
            self.assertIn(os.path.basename(wheel), body)


class MinimalEnvTest(unittest.TestCase):
    def test_drops_ambient_secrets_keeps_credentials(self):
        os.environ["PYPI_MINIMAL_ENV_PROBE"] = "ambient-secret"
        try:
            env = minimal_upload_env(
                {"TWINE_USERNAME": "__token__", "TWINE_PASSWORD": "tok"}
            )
        finally:
            del os.environ["PYPI_MINIMAL_ENV_PROBE"]
        self.assertNotIn("PYPI_MINIMAL_ENV_PROBE", env)
        self.assertEqual(env["TWINE_USERNAME"], "__token__")
        self.assertEqual(env["TWINE_PASSWORD"], "tok")
        self.assertIn("PATH", env)


if __name__ == "__main__":
    unittest.main()
