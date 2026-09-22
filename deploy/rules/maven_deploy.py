#!/usr/bin/env python3
"""Local file-repo builder plus gated Maven staging uploader for `maven_deploy`.

 Hermetic default builds a local file repo (`group/artifact/version/*.jar`
 plus the `.pom` with sha256 sidecars) and verifies bytes via sha256; the
 live `mvn deploy:deploy-file` staging path with GPG signing runs only with
 explicit env plus owner approval and never by default. The GPG passphrase
 never appears on the command line: it rides in the 0600 `settings.xml`
 profile properties, and the settings file is unlinked after the run.
 Unsigned staging is refused unless explicitly recorded with
 `MAVEN_ALLOW_UNSIGNED=1` plus owner approval. Used as an
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


def _xml_escape(text):
    """Escapes one value for embedding in the generated settings.xml."""
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
    )


def settings_xml(username, password, passphrase):
    """Renders the staging settings.xml with the secret values escaped.

    The GPG passphrase rides in the `dx-gpg-passphrase` profile
    properties (read by the `gpg-sign` profile's gpg plugin as
    `gpg.passphrase`), never on the Maven command line where `ps`
    could observe it. The profile block is omitted when no passphrase
    is given.
    """
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
    """Writes one 0600 file holding secret material, returning its path."""
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
    """Removes one secret-material directory, best effort."""
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
    """Stages jar plus pom to a remote repository without interactive prompts.

    GPG-signed by default via the passphrase in `MAVEN_GPG_PASSPHRASE`
    (kept out of `ps` in the 0600 settings file, unlinked afterwards).
    Without a passphrase the run is refused unless unsigned staging is
    explicitly recorded with `MAVEN_ALLOW_UNSIGNED=1` (owner approval
    is still required by the caller); the unsigned run passes
    `-Dgpg.skip=true` and records itself as unsigned.
    """
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
