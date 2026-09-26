#!/usr/bin/env python3
"""Local output test."""

import hashlib
import os
import tempfile
import unittest

from archive_deploy import sha256_file, stage_release


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class StageTest(unittest.TestCase):
    def test_verifies_checksum_and_copies_pair(self):
        with tempfile.TemporaryDirectory() as tmp:
            app = os.path.join(tmp, "deploy_program")
            with open(app, "wb") as f:
                f.write(b"deploy-program-bytes-v1")
            tarball = os.path.join(tmp, "release_demo.tar.gz")
            with open(tarball, "wb") as f:
                f.write(b"archive-tarball-bytes-v1")
            checksum = os.path.join(tmp, "release_demo.tar.gz.sha256")
            with open(checksum, "w", encoding="utf-8", newline="\n") as f:
                f.write(_sha256(tarball) + "  release_demo.tar.gz\n")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            app_name, tarball_dest, checksum_dest = stage_release(
                app, tarball, checksum, outdir
            )

            self.assertEqual(app_name, "deploy_program")
            for src, dest in ((tarball, tarball_dest), (checksum, checksum_dest)):
                self.assertTrue(os.path.isfile(dest), dest)
                self.assertEqual(_sha256(dest), _sha256(src))
            self.assertEqual(sha256_file(tarball_dest), _sha256(tarball))

    def test_rejects_checksum_mismatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            app = os.path.join(tmp, "deploy_program")
            with open(app, "wb") as f:
                f.write(b"deploy-program-bytes-v1")
            tarball = os.path.join(tmp, "release_demo.tar.gz")
            with open(tarball, "wb") as f:
                f.write(b"archive-tarball-bytes-v1")
            checksum = os.path.join(tmp, "release_demo.tar.gz.sha256")
            with open(checksum, "w", encoding="utf-8", newline="\n") as f:
                f.write("0" * 64 + "  release_demo.tar.gz\n")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(RuntimeError):
                stage_release(app, tarball, checksum, outdir)


if __name__ == "__main__":
    unittest.main()
