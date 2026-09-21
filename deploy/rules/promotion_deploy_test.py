#!/usr/bin/env python3
"""Local promotion staging test for `promotion_deploy` (no network, no sockets)."""

import hashlib
import json
import os
import tempfile
import unittest

from promotion_deploy import build_promotion, record_rollback


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _staged_files(root):
    found = []
    for dirpath, _, filenames in os.walk(root):
        for name in filenames:
            with open(os.path.join(dirpath, name), "rb") as f:
                found.append(f.read())
    return found


class PromotionTest(unittest.TestCase):
    def test_builds_promotion_dir_plus_record_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            artifact = os.path.join(tmp, "release_demo.tar.gz")
            with open(artifact, "wb") as f:
                f.write(b"promotion-demo-artifact-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            promotion = build_promotion(
                artifact,
                outdir,
                "promotion_demo",
                "staging",
                "production",
                "1.2.3",
                [],
            )

            self.assertEqual(promotion, os.path.join(outdir, "promotion_demo-promotion"))
            dest = os.path.join(promotion, os.path.basename(artifact))
            self.assertTrue(os.path.isfile(dest), dest)
            self.assertEqual(_sha256(dest), _sha256(artifact))
            record = os.path.join(promotion, "promotion.json")
            self.assertTrue(os.path.isfile(record), record)
            with open(record, encoding="utf-8") as f:
                body = json.load(f)
            self.assertEqual(body["from_environment"], "staging")
            self.assertEqual(body["to_environment"], "production")
            self.assertEqual(body["version"], "1.2.3")
            self.assertEqual(body["artifact_sha256"], _sha256(artifact))
            self.assertEqual(body["secret_refs"], [])
            manifest = os.path.join(promotion, "would-run.txt")
            self.assertTrue(os.path.isfile(manifest), manifest)
            with open(manifest, encoding="utf-8") as f:
                text = f.read()
            self.assertIn(
                "promote release_demo.tar.gz staging -> production version 1.2.3",
                text,
            )
            self.assertIn("secret-refs: none", text)
            self.assertIn(_sha256(artifact), text)

    def test_records_secret_names_without_values(self):
        with tempfile.TemporaryDirectory() as tmp:
            artifact = os.path.join(tmp, "release_demo.tar.gz")
            with open(artifact, "wb") as f:
                f.write(b"promotion-demo-artifact-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            canary = "canary-secret-value-9f8e7d6c5b4a"

            promotion = build_promotion(
                artifact,
                outdir,
                "promotion_demo",
                "staging",
                "production",
                "1.2.3",
                ["PROMOTION_API_TOKEN", "file:/run/secrets/app"],
            )

            with open(os.path.join(promotion, "promotion.json"), encoding="utf-8") as f:
                body = json.load(f)
            self.assertEqual(
                body["secret_refs"], ["PROMOTION_API_TOKEN", "file:/run/secrets/app"]
            )
            for blob in _staged_files(promotion):
                self.assertNotIn(canary.encode("utf-8"), blob)

    def test_rejects_self_edges(self):
        with tempfile.TemporaryDirectory() as tmp:
            artifact = os.path.join(tmp, "release_demo.tar.gz")
            with open(artifact, "wb") as f:
                f.write(b"promotion-demo-artifact-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            with self.assertRaises(ValueError):
                build_promotion(
                    artifact,
                    outdir,
                    "promotion_demo",
                    "production",
                    "production",
                    "1.2.3",
                    [],
                )

    def test_records_rollback_pointer_and_refuses_empty_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            artifact = os.path.join(tmp, "release_demo.tar.gz")
            with open(artifact, "wb") as f:
                f.write(b"promotion-demo-artifact-bytes-v1")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)
            promotion = build_promotion(
                artifact,
                outdir,
                "promotion_demo",
                "staging",
                "production",
                "1.2.4",
                [],
            )

            pointer = record_rollback(promotion, "promotion_demo", "1.2.3", "abc123")
            self.assertEqual(pointer, os.path.join(promotion, "rollback.txt"))
            with open(pointer, encoding="utf-8") as f:
                text = f.read()
            self.assertIn("rollback promotion_demo to 1.2.3", text)
            self.assertIn("abc123", text)
            # The staged forward artifact stays untouched.
            self.assertTrue(
                os.path.isfile(os.path.join(promotion, "release_demo.tar.gz"))
            )
            with self.assertRaises(ValueError):
                record_rollback(promotion, "promotion_demo", "")


if __name__ == "__main__":
    unittest.main()
