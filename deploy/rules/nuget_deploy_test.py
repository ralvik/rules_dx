#!/usr/bin/env python3
"""Local folder-feed output test for `nuget_deploy` (no network, no sockets)."""

import hashlib
import os
import tempfile
import unittest

from nuget_deploy import build_feed


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class FeedTest(unittest.TestCase):
    def test_builds_flat_feed_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            nupkg = os.path.join(tmp, "nuget_demo.0.0.0.nupkg")
            with open(nupkg, "wb") as f:
                f.write(b"nuget-demo-nupkg-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            feed = build_feed(nupkg, outdir, "nuget_demo")

            self.assertEqual(feed, os.path.join(outdir, "nuget_demo-feed"))
            dest = os.path.join(feed, os.path.basename(nupkg))
            self.assertTrue(os.path.isfile(dest), dest)
            self.assertEqual(_sha256(dest), _sha256(nupkg))

    def test_rejects_non_nupkg_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            other = os.path.join(tmp, "nuget_demo.0.0.0.zip")
            with open(other, "wb") as f:
                f.write(b"not-a-nupkg")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(ValueError):
                build_feed(other, outdir, "nuget_demo")


if __name__ == "__main__":
    unittest.main()
