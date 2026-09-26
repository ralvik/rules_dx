#!/usr/bin/env python3
"""Local package drop builder."""

import hashlib
import os
import shutil
import subprocess
import sys

PACKAGE_RLOC = "@@PACKAGE_RLOC@@"
DEPLOY_NAME = "@@DEPLOY_NAME@@"
PROJECT = "@@PROJECT@@"
CHANNEL = "@@CHANNEL@@"
VERSION = "@@VERSION@@"
DEPLOY_TO = "@@DEPLOY_TO@@"
SPACE = "@@SPACE@@"
OCTOPUS_URL = "@@OCTOPUS_URL@@"

OCTOPUS_PLACEHOLDER_URL = "https://octopus.example.invalid"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _split_deploy_to(raw):
    if not raw:
        return []
    return [p for p in raw.split(",") if p]


def push_command(package_base, server, space):
    parts = ["octo", "push", package_base, "--server", server]
    if space:
        parts += ["--space", space]
    return " ".join(parts)


def release_command(project, channel, version, package_base, deploy_to, space, server):
    parts = [
        "octo",
        "create-release",
        "--project",
        project,
        "--channel",
        channel,
        "--version",
        version,
        "--package",
        package_base,
    ]
    for env in deploy_to:
        parts += ["--deploy-to", env]
    if space:
        parts += ["--space", space]
    parts += ["--server", server]
    return " ".join(parts)


def build_drop(
    package_src,
    outdir,
    deploy_name,
    project,
    channel,
    version,
    deploy_to,
    space,
    server,
):
    if not package_src or not package_src.endswith(".tar.gz"):
        raise ValueError(
            "octopus drop: want exactly one .tar.gz source (archive_deploy output), got '"
            + str(package_src)
            + "'"
        )
    if not deploy_name:
        raise ValueError("octopus drop: need a non-empty deploy name")
    if not project:
        raise ValueError("octopus drop: need a non-empty project")
    if not channel:
        raise ValueError("octopus drop: need a non-empty channel")
    if not version:
        raise ValueError("octopus drop: need a non-empty version")

    drop = os.path.join(outdir, deploy_name + "-drop")
    os.makedirs(drop, exist_ok=True)
    base = os.path.basename(package_src)
    dest = os.path.join(drop, base)
    shutil.copyfile(package_src, dest)
    want = sha256_file(package_src)
    got = sha256_file(dest)
    if want != got:
        raise RuntimeError(
            "octopus drop: byte mismatch for "
            + base
            + " (expected sha256 "
            + want
            + ", got "
            + got
            + ")"
        )
    manifest = (
        "# would-run manifest for octopus_deploy (local default publishes nothing)\n"
        + push_command(base, server, space)
        + "\n"
        + release_command(project, channel, version, base, deploy_to, space, server)
        + "\n"
        + "package-sha256: "
        + want
        + "  "
        + base
        + "\n"
    )
    with open(
        os.path.join(drop, "would-run.txt"), "w", encoding="utf-8", newline="\n"
    ) as f:
        f.write(manifest)
    return drop


def live_push(package_src, server, api_key, space):
    cmd = ["octo", "push", package_src, "--server", server, "--apiKey", api_key]
    if space:
        cmd += ["--space", space]
    subprocess.run(cmd, check=True)


def live_release(
    project, channel, version, package_src, deploy_to, server, api_key, space
):
    cmd = [
        "octo",
        "create-release",
        "--project",
        project,
        "--channel",
        channel,
        "--version",
        version,
        "--package",
        os.path.basename(package_src),
        "--server",
        server,
        "--apiKey",
        api_key,
    ]
    for env in deploy_to:
        cmd += ["--deploy-to", env]
    if space:
        cmd += ["--space", space]
    subprocess.run(cmd, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("octopus_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "octopus_deploy: this deploy target takes at most an output directory; "
            "the drop is exactly the file pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    package = _resolve_runfiles(PACKAGE_RLOC)
    deploy_to = _split_deploy_to(DEPLOY_TO)
    space = os.environ.get("OCTOPUS_SPACE", SPACE)
    server = os.environ.get("OCTOPUS_URL", OCTOPUS_URL)

    if os.environ.get("OCTOPUS_PUBLISH_LIVE") == "1":
        api_key = os.environ.get("OCTOPUS_API_KEY", "")
        approved = os.environ.get("OCTOPUS_PUBLISH_APPROVED", "")
        if VERSION == "0.0.0":
            print(
                "octopus_deploy: live push refuses version 0.0.0; set a real version",
                file=sys.stderr,
            )
            return 1
        if not server or server == OCTOPUS_PLACEHOLDER_URL:
            print(
                "octopus_deploy: live push needs OCTOPUS_URL plus explicit "
                "owner approval (OCTOPUS_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if not api_key:
            print(
                "octopus_deploy: live push needs OCTOPUS_API_KEY plus explicit "
                "owner approval (OCTOPUS_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "octopus_deploy: live push needs OCTOPUS_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        live_push(package, server, api_key, space)
        live_release(
            PROJECT, CHANNEL, VERSION, package, deploy_to, server, api_key, space
        )
        print("octopus_deploy: pushed " + PROJECT + " " + VERSION)
        return 0

    drop = build_drop(
        package,
        outdir,
        DEPLOY_NAME,
        PROJECT,
        CHANNEL,
        VERSION,
        deploy_to,
        space,
        server,
    )
    print(
        "octopus_deploy: staged "
        + PROJECT
        + " "
        + VERSION
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  drop: " + drop)
    print("  manifest: " + os.path.join(drop, "would-run.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
