#!/usr/bin/env python3
"""Local output test."""

import hashlib
import json
import os
import tempfile
import unittest

from crates_deploy import build_vendor, minimal_publish_env


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class VendorTest(unittest.TestCase):
    def test_builds_vendor_plus_registry_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            manifest = os.path.join(tmp, "Cargo.toml")
            with open(manifest, "wb") as f:
                f.write(b'[package]\nname = "crates_demo"\nversion = "0.0.0"\n')
            lib = os.path.join(tmp, "lib.rs")
            with open(lib, "wb") as f:
                f.write(b'pub fn hello() -> &str { "hello" }\n')
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            house = build_vendor([manifest, lib], outdir, "crates_demo", "0.0.0")

            self.assertEqual(house, os.path.join(outdir, "crates_demo-vendor"))
            vendor_dir = os.path.join(house, "vendor", "crates_demo")
            registry_dir = os.path.join(house, "registry", "crates_demo", "0.0.0")
            for src in (manifest, lib):
                for dest_dir in (vendor_dir, registry_dir):
                    dest = os.path.join(dest_dir, os.path.basename(src))
                    self.assertTrue(os.path.isfile(dest), dest)
                    self.assertEqual(_sha256(dest), _sha256(src))
            checksum = os.path.join(vendor_dir, ".cargo-checksum.json")
            self.assertTrue(os.path.isfile(checksum), checksum)
            with open(checksum, encoding="utf-8") as f:
                payload = json.load(f)
            self.assertIn("Cargo.toml", payload["files"])
            self.assertIn("lib.rs", payload["files"])
            self.assertEqual(
                payload["files"]["Cargo.toml"], _sha256(manifest)
            )
            index = os.path.join(registry_dir, "index.json")
            self.assertTrue(os.path.isfile(index), index)
            with open(index, encoding="utf-8") as f:
                body = json.load(f)
            self.assertEqual(body["name"], "crates_demo")
            self.assertEqual(body["version"], "0.0.0")
            names = sorted(entry["name"] for entry in body["files"])
            self.assertEqual(names, ["Cargo.toml", "lib.rs"])

    def test_builds_single_file_vendor(self):
        with tempfile.TemporaryDirectory() as tmp:
            manifest = os.path.join(tmp, "Cargo.toml")
            with open(manifest, "wb") as f:
                f.write(b'[package]\nname = "crates_demo"\nversion = "0.0.0"\n')
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            house = build_vendor([manifest], outdir, "crates_demo", "0.0.1")

            vendor_dest = os.path.join(house, "vendor", "crates_demo", "Cargo.toml")
            self.assertTrue(os.path.isfile(vendor_dest), vendor_dest)
            self.assertEqual(_sha256(vendor_dest), _sha256(manifest))
            registry_dest = os.path.join(
                house, "registry", "crates_demo", "0.0.1", "Cargo.toml"
            )
            self.assertTrue(os.path.isfile(registry_dest), registry_dest)
            index = os.path.join(house, "registry", "crates_demo", "0.0.1", "index.json")
            with open(index, encoding="utf-8") as f:
                body = json.load(f)
            self.assertEqual(body["version"], "0.0.1")


class MinimalEnvTest(unittest.TestCase):
    def test_drops_ambient_secrets_keeps_token(self):
        os.environ["CRATES_MINIMAL_ENV_PROBE"] = "ambient-secret"
        try:
            env = minimal_publish_env({"CARGO_REGISTRY_TOKEN": "tok"})
        finally:
            del os.environ["CRATES_MINIMAL_ENV_PROBE"]
        self.assertNotIn("CRATES_MINIMAL_ENV_PROBE", env)
        self.assertEqual(env["CARGO_REGISTRY_TOKEN"], "tok")
        self.assertIn("PATH", env)


if __name__ == "__main__":
    unittest.main()
