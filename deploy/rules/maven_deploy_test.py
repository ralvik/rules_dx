#!/usr/bin/env python3
"""Local file-repo output test for `maven_deploy` (no network, no sockets)."""

import hashlib
import os
import stat
import tempfile
import unittest

from maven_deploy import build_file_repo, settings_xml, write_secure_file


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


class SettingsSecretsTest(unittest.TestCase):
    def test_passphrase_rides_in_settings_never_argv(self):
        body = settings_xml("user", "pass", "s3cret")
        self.assertIn("<gpg.passphrase>s3cret</gpg.passphrase>", body)
        self.assertIn("<activeProfile>dx-gpg-passphrase</activeProfile>", body)
        self.assertIn("<username>user</username>", body)

    def test_no_passphrase_omits_gpg_profile(self):
        body = settings_xml("user", "pass", "")
        self.assertNotIn("gpg.passphrase", body)
        self.assertNotIn("activeProfile", body)

    def test_credentials_are_xml_escaped(self):
        body = settings_xml("u&ser", "p<ass>", "a'b\"c")
        self.assertIn("<username>u&amp;ser</username>", body)
        self.assertIn("<password>p&lt;ass&gt;</password>", body)
        self.assertIn("<gpg.passphrase>a&apos;b&quot;c</gpg.passphrase>", body)

    def test_secure_file_is_owner_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = write_secure_file(tmp, "settings.xml", settings_xml("u", "p", "s"))
            mode = stat.S_IMODE(os.stat(path).st_mode)
            self.assertEqual(mode, 0o600)
            with open(path, encoding="utf-8") as f:
                self.assertIn("dx-staging", f.read())

    def test_unsigned_staging_refused_without_explicit_record(self):
        import maven_deploy

        os.environ.pop("MAVEN_GPG_PASSPHRASE", None)
        os.environ.pop("MAVEN_ALLOW_UNSIGNED", None)
        with tempfile.TemporaryDirectory() as tmp:
            jar = os.path.join(tmp, "a.jar")
            pom = os.path.join(tmp, "a.pom")
            for path in (jar, pom):
                with open(path, "wb") as f:
                    f.write(b"x")
            with self.assertRaises(RuntimeError):
                maven_deploy.live_deploy(
                    jar, pom, "g", "a", "1.0", "https://example.invalid", "u", "p"
                )


if __name__ == "__main__":
    unittest.main()
