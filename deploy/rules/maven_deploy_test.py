#!/usr/bin/env python3
"""Local file-repo output test for `maven_deploy` (no network, no sockets)."""

import hashlib
import os
import tempfile
import unittest

from maven_deploy import build_file_repo


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class FileRepoTest(unittest.TestCase):
    def test_builds_group_artifact_version_layout_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            jar = os.path.join(tmp, "maven_demo-0.0.0.jar")
            with open(jar, "wb") as f:
                f.write(b"maven-demo-jar-bytes-v1")
            pom = os.path.join(tmp, "maven_demo-0.0.0.pom")
            with open(pom, "wb") as f:
                f.write(b"<project>maven-demo-pom-v1</project>")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            house = build_file_repo(
                jar, pom, outdir, "com.example", "maven_demo", "0.0.0"
            )

            self.assertEqual(house, os.path.join(outdir, "maven_demo-repo"))
            dest_dir = os.path.join(house, "com", "example", "maven_demo", "0.0.0")
            jar_dest = os.path.join(dest_dir, "maven_demo-0.0.0.jar")
            pom_dest = os.path.join(dest_dir, "maven_demo-0.0.0.pom")
            for src, dest in ((jar, jar_dest), (pom, pom_dest)):
                self.assertTrue(os.path.isfile(dest), dest)
                self.assertEqual(_sha256(dest), _sha256(src))
                sidecar = dest + ".sha256"
                self.assertTrue(os.path.isfile(sidecar), sidecar)
                with open(sidecar, encoding="utf-8") as f:
                    body = f.read()
                self.assertIn(_sha256(src), body)
            metadata = os.path.join(
                house, "com", "example", "maven_demo", "maven-metadata.xml"
            )
            self.assertTrue(os.path.isfile(metadata), metadata)
            with open(metadata, encoding="utf-8") as f:
                body = f.read()
            self.assertIn("<groupId>com.example</groupId>", body)
            self.assertIn("<artifactId>maven_demo</artifactId>", body)
            self.assertIn("<version>0.0.0</version>", body)

    def test_rejects_empty_coordinates(self):
        with tempfile.TemporaryDirectory() as tmp:
            jar = os.path.join(tmp, "maven_demo-0.0.0.jar")
            with open(jar, "wb") as f:
                f.write(b"maven-demo-jar-bytes-v1")
            pom = os.path.join(tmp, "maven_demo-0.0.0.pom")
            with open(pom, "wb") as f:
                f.write(b"<project>maven-demo-pom-v1</project>")
            outdir = os.path.join(tmp, "out")
            os.makedirs(outdir)

            with self.assertRaises(ValueError):
                build_file_repo(jar, pom, outdir, "", "maven_demo", "0.0.0")


if __name__ == "__main__":
    unittest.main()
