#!/usr/bin/env python3
"""Local file repo builder."""

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile

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
    return group.replace(".", "/")


def build_file_repo(jar_src, pom_src, outdir, group, artifact, version):
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


def _xml_escape(text):
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
    )


def settings_xml(username, password, passphrase):
    body = (
        "<settings>\n"
        "  <servers>\n"
        "    <server>\n"
        "      <id>dx-staging</id>\n"
        "      <username>" + _xml_escape(username) + "</username>\n"
        "      <password>" + _xml_escape(password) + "</password>\n"
        "    </server>\n"
        "  </servers>\n"
    )
    if passphrase:
        body += (
            "  <profiles>\n"
            "    <profile>\n"
            "      <id>dx-gpg-passphrase</id>\n"
            "      <properties>\n"
            "        <gpg.passphrase>"
            + _xml_escape(passphrase)
            + "</gpg.passphrase>\n"
            "      </properties>\n"
            "    </profile>\n"
            "  </profiles>\n"
            "  <activeProfiles>\n"
            "    <activeProfile>dx-gpg-passphrase</activeProfile>\n"
            "  </activeProfiles>\n"
        )
    body += "</settings>\n"
    return body


def write_secure_file(directory, name, content):
    path = os.path.join(directory, name)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as f:
            f.write(content)
    except BaseException:
        try:
            os.unlink(path)
        except OSError:
            pass
        raise
    os.chmod(path, 0o600)
    return path


def _remove_tree(path):
    for root, dirs, files in os.walk(path, topdown=False):
        for name in files:
            try:
                os.unlink(os.path.join(root, name))
            except OSError:
                pass
        for name in dirs:
            try:
                os.rmdir(os.path.join(root, name))
            except OSError:
                pass
    try:
        os.rmdir(path)
    except OSError:
        pass


def live_deploy(
    jar_src, pom_src, group, artifact, version, repository_url, username, password
):
    passphrase = os.environ.get("MAVEN_GPG_PASSPHRASE", "")
    workdir = tempfile.mkdtemp(prefix="maven-deploy-")
    settings_file = write_secure_file(
        workdir, "settings.xml", settings_xml(username, password, passphrase)
    )
    try:
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
        if passphrase:
            cmd += ["-Pgpg-sign"]
        else:
            if os.environ.get("MAVEN_ALLOW_UNSIGNED", "") != "1":
                raise RuntimeError(
                    "maven staging: no MAVEN_GPG_PASSPHRASE; refusing unsigned publish "
                    "(set MAVEN_GPG_PASSPHRASE, or record unsigned staging explicitly "
                    "with MAVEN_ALLOW_UNSIGNED=1 plus owner approval)"
                )
            cmd += ["-Dgpg.skip=true"]
            print(
                "maven_deploy: unsigned staging recorded (MAVEN_ALLOW_UNSIGNED=1 "
                "with owner approval; no GPG signature attached)"
            )
        subprocess.run(cmd, check=True)
    finally:
        _remove_tree(workdir)


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
        try:
            live_deploy(
                jar, pom, GROUP, ARTIFACT, VERSION, REPOSITORY_URL, username, password
            )
        except (RuntimeError, subprocess.CalledProcessError) as e:
            print("maven_deploy: " + str(e), file=sys.stderr)
            return 1
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
