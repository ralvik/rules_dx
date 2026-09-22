#!/usr/bin/env python3
"""Local vendor builder plus gated crates.io uploader for `crates_deploy`.

 Hermetic default builds a local vendor directory (`vendor/` plus a file
 registry) and verifies bytes via sha256; the live `cargo publish` path
 runs only with explicit env plus owner approval and never by default.
 The live child inherits a minimal environment (PATH/HOME plus the
 registry token only), never the full parent env. Single-string registry
 tokens are the only supported credential: prefer short-lived tokens and
 rotate them per release; OIDC-based publish stays an owned gap until
 tooled. Used as an `expand_template` template per deploy instance
 (placeholders below) and as a `py_library` for `py_test`.
 """

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

# Per-instance pins expanded by the `crates_deploy` launcher rule. The
# checked-in placeholders keep this file importable for `py_test`, which
# exercises `build_vendor` directly without touching these constants.
CRATE_RLOCS_STR = "@@CRATE_RLOCS@@"
CRATE_NAME = "@@CRATE_NAME@@"
CRATE_VERSION = "@@CRATE_VERSION@@"
ALLOW_DIRTY = "@@ALLOW_DIRTY@@"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_vendor(crate_files, outdir, crate_name, version):
    """Copies crate sources into a local vendor plus file registry and verifies bytes.

    Creates `<outdir>/<crate>-vendor/` holding `vendor/<crate>/` (each
    source by basename plus `.cargo-checksum.json`) and
    `registry/<crate>/<version>/` (each source by basename plus
    `index.json` with name, version, and sha256 per file). Basenames must
    stay unique so the flattened vendor layout is deterministic. Returns
    the vendor house directory.
    """
    if not crate_files:
        raise ValueError("crates vendor: need at least one crate source file")
    seen = set()
    for src in crate_files:
        base = os.path.basename(src)
        if not base:
            raise ValueError("crates vendor: empty basename for '" + src + "'")
        if base in seen:
            raise ValueError(
                "crates vendor: duplicate basename '" + base + "' (keep basenames unique)"
            )
        seen.add(base)

    house = os.path.join(outdir, crate_name + "-vendor")
    vendor_dir = os.path.join(house, "vendor", crate_name)
    registry_dir = os.path.join(house, "registry", crate_name, version)
    os.makedirs(vendor_dir, exist_ok=True)
    os.makedirs(registry_dir, exist_ok=True)

    entries = []
    for src in sorted(crate_files):
        base = os.path.basename(src)
        want = sha256_file(src)
        for dest_dir in (vendor_dir, registry_dir):
            dest = os.path.join(dest_dir, base)
            shutil.copyfile(src, dest)
            got = sha256_file(dest)
            if want != got:
                raise RuntimeError(
                    "crates vendor: byte mismatch for "
                    + base
                    + " (expected sha256 "
                    + want
                    + ", got "
                    + got
                    + ")"
                )
        entries.append((base, want))

    package_digest = hashlib.sha256(
        "\n".join(base + ":" + digest for base, digest in sorted(entries)).encode(
            "utf-8"
        )
    ).hexdigest()
    with open(
        os.path.join(vendor_dir, ".cargo-checksum.json"),
        "w",
        encoding="utf-8",
        newline="\n",
    ) as f:
        json.dump(
            {
                "files": {base: digest for base, digest in sorted(entries)},
                "package": package_digest,
            },
            f,
            indent=2,
            sort_keys=True,
        )
        f.write("\n")

    with open(
        os.path.join(registry_dir, "index.json"),
        "w",
        encoding="utf-8",
        newline="\n",
    ) as f:
        json.dump(
            {
                "files": [
                    {"name": base, "sha256": digest}
                    for base, digest in sorted(entries)
                ],
                "name": crate_name,
                "version": version,
            },
            f,
            indent=2,
            sort_keys=True,
        )
        f.write("\n")
    return house


def minimal_publish_env(extra):
    """Builds the minimal child environment for a registry publisher.

    Carries locale/PATH/HOME/TMP plus exactly the credential entries in
    `extra`; every other parent variable (ambient secrets, proxies,
    configuration overrides) is dropped. Mirrors
    `pypi_deploy.minimal_upload_env` so the two uploaders stay in sync.
    """
    keep = (
        "HOME",
        "LANG",
        "LC_ALL",
        "PATH",
        "TMPDIR",
        "USER",
        "LOGNAME",
        "SystemRoot",
        "SystemDrive",
        "PATHEXT",
        "CARGO_HOME",
        "RUSTUP_HOME",
    )
    env = {key: os.environ[key] for key in keep if key in os.environ}
    env.update(extra)
    return env


def live_publish(crate_files, token):
    """Publishes staged crate sources via cargo publish without dirty trees."""
    if os.environ.get("CRATES_ALLOW_DIRTY") == "1":
        raise RuntimeError(
            "crates publish: --allow-dirty rejected by default; publish from a clean tree"
        )
    stage = tempfile.mkdtemp(prefix="crates-publish-")
    for src in crate_files:
        shutil.copyfile(src, os.path.join(stage, os.path.basename(src)))
    manifest = os.path.join(stage, "Cargo.toml")
    if not os.path.isfile(manifest):
        raise RuntimeError(
            "crates publish: staged crate has no Cargo.toml (need one among crate sources)"
        )
    env = minimal_publish_env({"CARGO_REGISTRY_TOKEN": token})
    cmd = ["cargo", "publish", "--manifest-path", manifest]
    subprocess.run(cmd, env=env, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("crates_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "crates_deploy: this deploy target takes at most an output directory; "
            "the vendor tree is exactly the files pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    rlocs = [p for p in CRATE_RLOCS_STR.split(";") if p]
    crate_files = [_resolve_runfiles(rloc) for rloc in rlocs]

    if os.environ.get("CRATES_PUBLISH_LIVE") == "1":
        if ALLOW_DIRTY == "1":
            print(
                "crates_deploy: --allow-dirty rejected by default; publish "
                "from a clean tree instead",
                file=sys.stderr,
            )
            return 1
        if os.environ.get("CRATES_ALLOW_DIRTY") == "1":
            print(
                "crates_deploy: --allow-dirty rejected by default; publish "
                "from a clean tree instead",
                file=sys.stderr,
            )
            return 1
        token = os.environ.get("CARGO_REGISTRY_TOKEN", "")
        approved = os.environ.get("CRATES_PUBLISH_APPROVED", "")
        if CRATE_VERSION == "0.0.0":
            print(
                "crates_deploy: live publish refuses version 0.0.0; set a real version",
                file=sys.stderr,
            )
            return 1
        if not token:
            print(
                "crates_deploy: live publish needs CARGO_REGISTRY_TOKEN plus "
                "explicit owner approval (CRATES_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "crates_deploy: live publish needs CRATES_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        live_publish(crate_files, token)
        print("crates_deploy: published " + CRATE_NAME + " " + CRATE_VERSION)
        return 0

    house = build_vendor(crate_files, outdir, CRATE_NAME, CRATE_VERSION)
    print(
        "crates_deploy: staged "
        + CRATE_NAME
        + " "
        + CRATE_VERSION
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  vendor: " + os.path.join(house, "vendor", CRATE_NAME))
    print(
        "  registry: " + os.path.join(house, "registry", CRATE_NAME, CRATE_VERSION)
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
