#!/usr/bin/env python3
"""Local output test."""

import hashlib
import json
import os
import tempfile
import unittest

from oci_deploy import build_layout


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class LayoutTest(unittest.TestCase):
    def test_builds_layout_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            tar = os.path.join(tmp, "oci_demo.tar")
            with open(tar, "wb") as f:
                f.write(b"oci-demo-image-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            layout = build_layout(tar, outdir, "ghcr.io", "oci_demo", "latest")

            self.assertEqual(layout, os.path.join(outdir, "oci_demo-oci-layout"))
            with open(os.path.join(layout, "oci-layout"), encoding="utf-8") as f:
                body = json.load(f)
            self.assertEqual(body["imageLayoutVersion"], "1.0.0")
            index_path = os.path.join(layout, "index.json")
            self.assertTrue(os.path.isfile(index_path), index_path)
            with open(index_path, encoding="utf-8") as f:
                index = json.load(f)
            self.assertEqual(index["schemaVersion"], 2)
            self.assertEqual(len(index["manifests"]), 1)
            manifest_desc = index["manifests"][0]
            self.assertEqual(
                manifest_desc["annotations"]["org.opencontainers.image.ref.name"],
                "ghcr.io/oci_demo:latest",
            )
            manifest_blob = os.path.join(
                layout, "blobs", "sha256", manifest_desc["digest"].replace("sha256:", "")
            )
            self.assertTrue(os.path.isfile(manifest_blob), manifest_blob)
            with open(manifest_blob, encoding="utf-8") as f:
                manifest = json.load(f)
            self.assertEqual(manifest["schemaVersion"], 2)
            layer_desc = manifest["layers"][0]
            layer_blob = os.path.join(
                layout, "blobs", "sha256", layer_desc["digest"].replace("sha256:", "")
            )
            self.assertTrue(os.path.isfile(layer_blob), layer_blob)
            self.assertEqual(_sha256(layer_blob), _sha256(tar))
            config_desc = manifest["config"]
            config_blob = os.path.join(
                layout, "blobs", "sha256", config_desc["digest"].replace("sha256:", "")
            )
            self.assertTrue(os.path.isfile(config_blob), config_blob)

    def test_rejects_non_tar_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            other = os.path.join(tmp, "oci_demo.zip")
            with open(other, "wb") as f:
                f.write(b"not-a-tar")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(ValueError):
                build_layout(other, outdir, "ghcr.io", "oci_demo", "latest")


if __name__ == "__main__":
    unittest.main()
