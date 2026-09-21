#!/usr/bin/env python3
"""Local OCI layout builder plus gated registry uploader for `oci_deploy`.

Hermetic default builds a local OCI image-layout directory
(`oci-layout`, `index.json`, `blobs/sha256/`) from the pinned image tar
and verifies bytes via sha256 with no daemon; the live push path runs
only with explicit env plus owner approval and never by default. Used as
an `expand_template` template per deploy instance (placeholders below)
and as a `py_library` for `py_test`.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys

# Per-instance pins expanded by the `oci_deploy` launcher rule. The
# checked-in placeholders keep this file importable for `py_test`, which
# exercises `build_layout` directly without touching these constants.
IMAGE_RLOC = "@@IMAGE_RLOC@@"
OCI_REGISTRY = "@@OCI_REGISTRY@@"
OCI_REPOSITORY = "@@OCI_REPOSITORY@@"
OCI_TAG = "@@OCI_TAG@@"

OCI_LAYOUT_VERSION = "1.0.0"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_json(path, payload):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def _blob_path(layout, digest):
    return os.path.join(layout, "blobs", "sha256", digest.replace("sha256:", ""))


def build_layout(image_tar_src, outdir, registry, repository, tag):
    """Converts one image tar into a local OCI image layout and verifies bytes.

    Creates `<outdir>/<repo-basename>-oci-layout/` holding `oci-layout`,
    `index.json`, and `blobs/sha256/` (layer plus generated config and
    manifest). The layer blob is the pinned tar bytes; config and
    manifest are deterministic JSON (sorted keys, fixed timestamps).
    Returns the layout directory.
    """
    if not image_tar_src or not (
        image_tar_src.endswith(".tar") or image_tar_src.endswith(".tar.gz")
    ):
        raise ValueError(
            "oci layout: want exactly one .tar source, got '" + str(image_tar_src) + "'"
        )
    if not registry or "://" in registry:
        raise ValueError("oci layout: want a registry host, got '" + str(registry) + "'")
    if not repository or repository.startswith("/") or repository.endswith("/"):
        raise ValueError(
            "oci layout: want a repository path, got '" + str(repository) + "'"
        )
    if not tag:
        raise ValueError("oci layout: want a non-empty tag")
    base = repository.split("/")[-1]
    if not base:
        raise ValueError(
            "oci layout: want a non-empty repository basename, got '"
            + str(repository)
            + "'"
        )
    layout = os.path.join(outdir, base + "-oci-layout")
    blobs = os.path.join(layout, "blobs", "sha256")
    os.makedirs(blobs, exist_ok=True)

    layer_size = os.path.getsize(image_tar_src)
    layer_digest = "sha256:" + sha256_file(image_tar_src)
    layer_blob = _blob_path(layout, layer_digest)
    shutil.copyfile(image_tar_src, layer_blob)
    if sha256_file(layer_blob) != layer_digest.replace("sha256:", ""):
        raise RuntimeError("oci layout: byte mismatch for layer blob")
    if image_tar_src.endswith(".tar.gz"):
        layer_media = "application/vnd.oci.image.layer.v1.tar+gzip"
    else:
        layer_media = "application/vnd.oci.image.layer.v1.tar"

    config = {
        "architecture": "amd64",
        "created": "1970-01-01T00:00:00Z",
        "os": "linux",
        "rootfs": {"diff_ids": [layer_digest], "type": "layers"},
    }
    config_bytes = (json.dumps(config, sort_keys=True) + "\n").encode("utf-8")
    config_digest = "sha256:" + hashlib.sha256(config_bytes).hexdigest()
    with open(_blob_path(layout, config_digest), "wb") as f:
        f.write(config_bytes)

    manifest = {
        "config": {
            "digest": config_digest,
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "size": len(config_bytes),
        },
        "layers": [
            {
                "digest": layer_digest,
                "mediaType": layer_media,
                "size": layer_size,
            }
        ],
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "schemaVersion": 2,
    }
    manifest_bytes = (json.dumps(manifest, sort_keys=True) + "\n").encode("utf-8")
    manifest_digest = "sha256:" + hashlib.sha256(manifest_bytes).hexdigest()
    with open(_blob_path(layout, manifest_digest), "wb") as f:
        f.write(manifest_bytes)

    ref = registry + "/" + repository + ":" + tag
    index = {
        "manifests": [
            {
                "annotations": {"org.opencontainers.image.ref.name": ref},
                "digest": manifest_digest,
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "size": len(manifest_bytes),
            }
        ],
        "mediaType": "application/vnd.oci.image.index.v1+json",
        "schemaVersion": 2,
    }
    _write_json(os.path.join(layout, "index.json"), index)
    _write_json(os.path.join(layout, "oci-layout"), {"imageLayoutVersion": OCI_LAYOUT_VERSION})
    return layout


def live_push(image_tar_src, layout, registry, repository, tag):
    """Pushes one pinned image tar via docker without interactive prompts."""
    ref = registry + "/" + repository + ":" + tag
    user = os.environ.get("OCI_REGISTRY_USER", "")
    token = os.environ.get("OCI_REGISTRY_TOKEN", "")
    if not user or not token:
        raise RuntimeError(
            "oci push: live push needs OCI_REGISTRY_USER plus OCI_REGISTRY_TOKEN"
        )
    login = subprocess.run(
        ["docker", "login", registry, "-u", user, "--password-stdin"],
        input=(token + "\n").encode("utf-8"),
        check=False,
    )
    if login.returncode != 0:
        raise RuntimeError("oci push: docker login failed for '" + registry + "'")
    subprocess.run(["docker", "load", "-i", image_tar_src], check=True)
    subprocess.run(["docker", "push", ref], check=True)
    return ref


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("oci_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "oci_deploy: this deploy target takes at most an output directory; "
            "the layout is exactly the file pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    image_tar = _resolve_runfiles(IMAGE_RLOC)

    if os.environ.get("OCI_PUBLISH_LIVE") == "1":
        user = os.environ.get("OCI_REGISTRY_USER", "")
        token = os.environ.get("OCI_REGISTRY_TOKEN", "")
        approved = os.environ.get("OCI_PUBLISH_APPROVED", "")
        if OCI_TAG in ("0.0.0", "0.0.0-dryrun"):
            print(
                "oci_deploy: live push refuses placeholder tag; set a real tag",
                file=sys.stderr,
            )
            return 1
        if not user or not token:
            print(
                "oci_deploy: live push needs OCI_REGISTRY_USER plus "
                "OCI_REGISTRY_TOKEN plus explicit owner approval "
                "(OCI_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "oci_deploy: live push needs OCI_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        if os.environ.get("OCI_PUBLISH_DRY_RUN") == "1":
            print(
                "oci_deploy: dry run (OCI_PUBLISH_DRY_RUN=1); would push "
                + OCI_REGISTRY
                + "/"
                + OCI_REPOSITORY
                + ":"
                + OCI_TAG
            )
            print(
                "oci_deploy: signing-first: verify SBOM/provenance plus "
                "signing_demo before any live push, then cosign sign <digest>"
            )
            return 0
        if shutil.which("docker") is None:
            print("oci_deploy: 'docker' CLI not found on PATH", file=sys.stderr)
            return 1
        layout = build_layout(
            image_tar, outdir, OCI_REGISTRY, OCI_REPOSITORY, OCI_TAG
        )
        ref = live_push(image_tar, layout, OCI_REGISTRY, OCI_REPOSITORY, OCI_TAG)
        print("oci_deploy: pushed " + ref)
        print(
            "oci_deploy: signing-first: run cosign sign <digest> plus verify "
            "on the release trust root before updating scaffold refs"
        )
        return 0

    layout = build_layout(image_tar, outdir, OCI_REGISTRY, OCI_REPOSITORY, OCI_TAG)
    print(
        "oci_deploy: staged "
        + OCI_REGISTRY
        + "/"
        + OCI_REPOSITORY
        + ":"
        + OCI_TAG
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  layout: " + layout)
    print("  index: " + os.path.join(layout, "index.json"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
