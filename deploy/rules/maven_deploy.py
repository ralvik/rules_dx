#!/usr/bin/env python3
"""Local file-repo builder plus gated Maven staging uploader for `maven_deploy`.

Hermetic default builds a local file repo (`group/artifact/version/*.jar`
plus the `.pom` with sha256 sidecars) and verifies bytes via sha256; the
live `mvn deploy:deploy-file` staging path with GPG signing runs only with
explicit env plus owner approval and never by default. Used as an
`expand_template` template per deploy instance (placeholders below) and as
a `py_library` for `py_test`.
"""

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile

# Per-instance pins expanded by the `maven_deploy` launcher rule. The
# checked-in placeholders keep this file importable for `py_test`, which
# exercises `build_file_repo` directly without touching these constants.
JAR_RLOC = "@@JAR_RLOC@@"
POM_RLOC = "@@POM_RLOC@@"
GROUP = "@@GROUP@@"
ARTIFACT = "@@ARTIFACT@@"
VERSION = "@@VERSION@@"
REPOSITORY_URL = "@@REPOSITORY_URL@@"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def group_path(group):
    """Converts a dotted groupId to its repository path."""
    return group.replace(".", "/")


def build_file_repo(jar_src, pom_src, outdir, group, artifact, version):
    """Copies jar plus pom into a local file repo and verifies bytes.

    Creates `<outdir>/<artifact>-repo/` holding
    `<group-path>/<artifact>/<version>/` (the canonical
    `<artifact>-<version>.jar` plus `<artifact>-<version>.pom`, each with
    a `.sha256` sidecar) and
    `<group-path>/<artifact>/maven-metadata.xml` pinning the single
    shipped version. Returns the repo house directory.
    """
    if not group:
        raise ValueError("maven file repo: need a non-empty groupId")
    if not artifact:
        raise ValueError("maven file repo: need a non-empty artifactId")
    if not version:
        raise ValueError("maven file repo: need a non-empty version")
    if not jar_src or not pom_src:
        raise ValueError("maven file repo: need both a jar and a pom source")

    house = os.path.join(outdir, artifact + "-repo")
    dest_dir = os.path.join(house, group_path(group), artifact, version)
    os.makedirs(dest_dir, exist_ok=True)

    jar_base = artifact + "-" + version + ".jar"
    pom_base = artifact + "-" + version + ".pom"
    pairs = [(jar_src, jar_base), (pom_src, pom_base)]
    for src, base in pairs:
        dest = os.path.join(dest_dir, base)
        shutil.copyfile(src, dest)
        want = sha256_file(src)
        got = sha256_file(dest)
        if want != got:
            raise RuntimeError(
                "maven file repo: byte mismatch for "
                + base
                + " (expected sha256 "
                + want
                + ", got "
                + got
                + ")"
            )
        with open(dest + ".sha256", "w", encoding="utf-8", newline="\n") as f:
            f.write(want + "  " + base + "\n")

    metadata_dir = os.path.join(house, group_path(group), artifact)
    os.makedirs(metadata_dir, exist_ok=True)
    metadata = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        "<metadata>\n"
        "  <groupId>" + group + "</groupId>\n"
        "  <artifactId>" + artifact + "</artifactId>\n"
        "  <version>" + version + "</version>\n"
        "  <versioning>\n"
        "    <latest>" + version + "</latest>\n"
        "    <release>" + version + "</release>\n"
        "    <versions>\n"
        "      <version>" + version + "</version>\n"
        "    </versions>\n"
        "  </versioning>\n"
        "</metadata>\n"
    )
    with open(
        os.path.join(metadata_dir, "maven-metadata.xml"),
        "w",
        encoding="utf-8",
        newline="\n",
    ) as f:
        f.write(metadata)
    return house


def live_deploy(
    jar_src, pom_src, group, artifact, version, repository_url, username, password
):
    """Stages jar plus pom to a remote repository without interactive prompts."""
    settings = (
        "<settings>\n"
        "  <servers>\n"
        "    <server>\n"
        "      <id>dx-staging</id>\n"
        "      <username>" + username + "</username>\n"
        "      <password>" + password + "</password>\n"
        "    </server>\n"
        "  </servers>\n"
        "</settings>\n"
    )
    settings_file = os.path.join(
        tempfile.mkdtemp(prefix="maven-deploy-"), "settings.xml"
    )
    with open(settings_file, "w", encoding="utf-8", newline="\n") as f:
        f.write(settings)
    cmd = [
        "mvn",
        "-B",
        "--settings",
        settings_file,
        "deploy:deploy-file",
        "-Durl=" + repository_url,
        "-DrepositoryId=dx-staging",
        "-Dfile=" + jar_src,
        "-DpomFile=" + pom_src,
        "-DgroupId=" + group,
        "-DartifactId=" + artifact,
        "-Dversion=" + version,
        "-Dpackaging=jar",
    ]
    passphrase = os.environ.get("MAVEN_GPG_PASSPHRASE", "")
    if passphrase:
        cmd += ["-Pgpg-sign", "-Dgpg.passphrase=" + passphrase]
    subprocess.run(cmd, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("maven_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "maven_deploy: this deploy target takes at most an output directory; "
            "the file repo is exactly the files pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    jar = _resolve_runfiles(JAR_RLOC)
    pom = _resolve_runfiles(POM_RLOC)

    if os.environ.get("MAVEN_PUBLISH_LIVE") == "1":
        username = os.environ.get("MAVEN_USERNAME", "")
        password = os.environ.get("MAVEN_PASSWORD", "")
        approved = os.environ.get("MAVEN_PUBLISH_APPROVED", "")
        if VERSION == "0.0.0":
            print(
                "maven_deploy: live staging refuses version 0.0.0; set a real version",
                file=sys.stderr,
            )
            return 1
        if not username or not password:
            print(
                "maven_deploy: live staging needs MAVEN_USERNAME plus "
                "MAVEN_PASSWORD with explicit owner approval "
                "(MAVEN_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "maven_deploy: live staging needs MAVEN_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        live_deploy(
            jar, pom, GROUP, ARTIFACT, VERSION, REPOSITORY_URL, username, password
        )
        print("maven_deploy: staged " + GROUP + ":" + ARTIFACT + ":" + VERSION)
        return 0

    house = build_file_repo(jar, pom, outdir, GROUP, ARTIFACT, VERSION)
    print(
        "maven_deploy: staged "
        + GROUP
        + ":"
        + ARTIFACT
        + ":"
        + VERSION
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  repo: " + house)
    print("  url: file://" + house)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
