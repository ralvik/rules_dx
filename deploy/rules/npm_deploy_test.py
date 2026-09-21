"""Pack plus feed output test for `npm_deploy`."""

import hashlib
import json
import os
import tarfile
import tempfile
import unittest

import npm_deploy
import npm_pack


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


class PackFeedTest(unittest.TestCase):
    def test_pack_and_feed(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "package.json")
            with open(src, "w", encoding="utf-8", newline="\n") as f:
                f.write('{"name": "npm-demo", "version": "0.0.0"}\n')
            tgz = os.path.join(tmp, "npm_demo.tgz")
            feed = os.path.join(tmp, "npm_demo.feed.json")
            digest = npm_pack.create_pack(
                "npm-demo",
                "latest",
                "https://registry.npmjs.org",
                tgz,
                feed,
                [src],
            )
            self.assertEqual(digest, sha256_file(tgz))
            with open(feed, encoding="utf-8") as f:
                data = json.load(f)
            self.assertEqual(data["name"], "npm-demo")
            self.assertEqual(data["tag"], "latest")
            self.assertEqual(data["registry"], "https://registry.npmjs.org")
            self.assertEqual(data["tarball"], "npm_demo.tgz")
            self.assertEqual(data["sha256"], digest)
            self.assertEqual(data["files"], ["package/package.json"])
            with tarfile.open(tgz, "r:gz") as tf:
                self.assertEqual(tf.getnames(), ["package/package.json"])
            verified = npm_deploy.verify_pack(tgz, feed)
            self.assertEqual(verified["sha256"], digest)
            tgz2 = os.path.join(tmp, "npm_demo2.tgz")
            feed2 = os.path.join(tmp, "npm_demo2.feed.json")
            digest2 = npm_pack.create_pack(
                "npm-demo",
                "latest",
                "https://registry.npmjs.org",
                tgz2,
                feed2,
                [src],
            )
            with open(tgz, "rb") as f1, open(tgz2, "rb") as f2:
                self.assertEqual(f1.read(), f2.read())
            self.assertEqual(digest, digest2)


if __name__ == "__main__":
    unittest.main()
