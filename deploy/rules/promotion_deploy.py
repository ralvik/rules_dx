#!/usr/bin/env python3
"""Local promotion stager."""

import hashlib
import json
import os
import shlex
import shutil
import subprocess
import sys

ARTIFACT_RLOC = "@@ARTIFACT_RLOC@@"
DEPLOY_NAME = "@@DEPLOY_NAME@@"
FROM_ENV = "@@FROM_ENV@@"
TO_ENV = "@@TO_ENV@@"
VERSION = "@@VERSION@@"

PROMOTION_PLACEHOLDER_VERSION = "0.0.0"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _split_refs(raw):
    if not raw:
        return []
    return [p.strip() for p in raw.split(",") if p.strip()]


def promote_command(artifact_base, from_env, to_env, version):
    return (
        "promote "
        + artifact_base
        + " "
        + from_env
        + " -> "
        + to_env
        + " version "
        + version
    )


def rollback_command(deploy_name, rollback_to):
    return (
        "rollback "
        + deploy_name
        + " to "
        + rollback_to
        + " (restore the pinned artifact for that version)"
    )


def build_promotion(
    artifact_src,
    outdir,
    deploy_name,
    from_env,
    to_env,
    version,
    secret_refs,
):
    if not artifact_src or not os.path.isfile(artifact_src):
        raise ValueError(
            "promotion: want exactly one existing artifact source, got '"
            + str(artifact_src)
            + "'"
        )
    if not deploy_name:
        raise ValueError("promotion: need a non-empty deploy name")
    if not from_env:
        raise ValueError("promotion: need a non-empty source environment")
    if not to_env:
        raise ValueError("promotion: need a non-empty target environment")
    if from_env == to_env:
        raise ValueError(
            "promotion: source and target environments must differ, got '"
            + from_env
            + "'"
        )
    if not version:
        raise ValueError("promotion: need a non-empty version")

    promotion = os.path.join(outdir, deploy_name + "-promotion")
    os.makedirs(promotion, exist_ok=True)
    base = os.path.basename(artifact_src)
    dest = os.path.join(promotion, base)
    shutil.copyfile(artifact_src, dest)
    want = sha256_file(artifact_src)
    got = sha256_file(dest)
    if want != got:
        raise RuntimeError(
            "promotion: byte mismatch for "
            + base
            + " (expected sha256 "
            + want
            + ", got "
            + got
            + ")"
        )
    refs = list(secret_refs) if secret_refs else []
    record = {
        "artifact": base,
        "artifact_sha256": want,
        "deploy": deploy_name,
        "from_environment": from_env,
        "health": "skipped-local",
        "registry_auth": "env-only",
        "secret_refs": sorted(refs),
        "to_environment": to_env,
        "version": version,
    }
    with open(
        os.path.join(promotion, "promotion.json"),
        "w",
        encoding="utf-8",
        newline="\n",
    ) as f:
        f.write(json.dumps(record, indent=2, sort_keys=True) + "\n")
    if refs:
        refs_line = "secret-refs: " + ",".join(sorted(refs)) + " (names only, never values)"
    else:
        refs_line = "secret-refs: none"
    manifest = (
        "# would-run manifest for promotion_deploy "
        "(local default publishes nothing; live needs PROMOTION_LIVE=1 plus PROMOTION_APPROVED=1)\n"
        + promote_command(base, from_env, to_env, version)
        + "\n"
        + "health: skipped locally; set PROMOTION_REQUIRE_HEALTH=1 plus "
        + "PROMOTION_HEALTH_CMD to gate the promotion on a health check\n"
        + "rollback: set PROMOTION_ROLLBACK=1 plus PROMOTION_ROLLBACK_TO=<version> "
        + "to record a rollback pointer (history lives in the promotion dir or registry)\n"
        + "artifact-sha256: "
        + want
        + "  "
        + base
        + "\n"
        + refs_line
        + "\n"
        + "registry-auth: env-only (PROMOTION_REGISTRY_USER/PROMOTION_REGISTRY_TOKEN), never from BUILD\n"
    )
    with open(
        os.path.join(promotion, "would-run.txt"), "w", encoding="utf-8", newline="\n"
    ) as f:
        f.write(manifest)
    return promotion


def record_rollback(promotion_dir, deploy_name, rollback_to, rollback_sha=""):
    if not promotion_dir or not os.path.isdir(promotion_dir):
        raise ValueError(
            "promotion rollback: want an existing promotion directory, got '"
            + str(promotion_dir)
            + "'"
        )
    if not rollback_to:
        raise ValueError("promotion rollback: need PROMOTION_ROLLBACK_TO=<version>")
    body = (
        "# rollback pointer for promotion_deploy (restores, never deletes)\n"
        + rollback_command(deploy_name, rollback_to)
        + "\n"
    )
    if rollback_sha:
        body += "expected-sha256: " + rollback_sha + "\n"
    path = os.path.join(promotion_dir, "rollback.txt")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(body)
    return path


def run_health_cmd(health_cmd):
    argv = shlex.split(health_cmd)
    if not argv:
        raise ValueError("promotion health: need a non-empty PROMOTION_HEALTH_CMD")
    if shutil.which(argv[0]) is None:
        raise RuntimeError(
            "promotion health: command not found on PATH: '" + argv[0] + "'"
        )
    proc = subprocess.run(argv, check=False)
    return proc.returncode


def _check_secret_refs(secret_refs):
    normalized = []
    for ref in secret_refs:
        if ref.startswith("file:"):
            path = ref[len("file:") :]
            if not path or not os.path.exists(path):
                raise RuntimeError(
                    "promotion secrets: secret file not found for ref '" + ref + "'"
                )
            normalized.append(ref)
        elif ref.startswith("cmd:"):
            tool = ref[len("cmd:") :].split(" ", 1)[0]
            if not tool or shutil.which(tool) is None:
                raise RuntimeError(
                    "promotion secrets: secret tool not installed for ref '"
                    + ref
                    + "' (install it separately and accept its license first)"
                )
            normalized.append(ref)
        elif not ref or any(c in ref for c in (" ", "\n", "'", '"', "\\", "$", "`")):
            raise ValueError(
                "promotion secrets: want ENV_NAME, file:<path>, or cmd:<tool> refs, got '"
                + ref
                + "'"
            )
        else:
            if not os.environ.get(ref):
                raise RuntimeError(
                    "promotion secrets: env ref '" + ref + "' is not set"
                )
            normalized.append(ref)
    return normalized


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("promotion_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "promotion_deploy: this deploy target takes at most an output directory; "
            "the promotion is exactly the file pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    artifact = _resolve_runfiles(ARTIFACT_RLOC)
    secret_refs = _split_refs(os.environ.get("PROMOTION_SECRET_REFS", ""))

    if os.environ.get("PROMOTION_ROLLBACK") == "1":
        promotion = build_promotion(
            artifact,
            outdir,
            DEPLOY_NAME,
            FROM_ENV,
            TO_ENV,
            VERSION,
            secret_refs,
        )
        try:
            rollback_to = os.environ.get("PROMOTION_ROLLBACK_TO", "")
            rollback_sha = os.environ.get("PROMOTION_ROLLBACK_SHA", "")
            record_rollback(promotion, DEPLOY_NAME, rollback_to, rollback_sha)
        except (ValueError, RuntimeError) as e:
            print("promotion_deploy: " + str(e), file=sys.stderr)
            return 1
        print(
            "promotion_deploy: recorded rollback for "
            + DEPLOY_NAME
            + " to "
            + os.environ.get("PROMOTION_ROLLBACK_TO", "")
        )
        print("  promotion: " + promotion)
        return 0

    if os.environ.get("PROMOTION_LIVE") == "1":
        approved = os.environ.get("PROMOTION_APPROVED", "")
        if VERSION == PROMOTION_PLACEHOLDER_VERSION:
            print(
                "promotion_deploy: live promote refuses version 0.0.0; set a real version",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "promotion_deploy: live promote needs PROMOTION_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        if os.environ.get("PROMOTION_REGISTRY_REQUIRED") == "1":
            if not os.environ.get(
                "PROMOTION_REGISTRY_USER", ""
            ) or not os.environ.get("PROMOTION_REGISTRY_TOKEN", ""):
                print(
                    "promotion_deploy: live promote needs PROMOTION_REGISTRY_USER plus "
                    "PROMOTION_REGISTRY_TOKEN (env only, never BUILD); refusing",
                    file=sys.stderr,
                )
                return 1
        try:
            _check_secret_refs(secret_refs)
        except (ValueError, RuntimeError) as e:
            print("promotion_deploy: " + str(e), file=sys.stderr)
            return 1
        if os.environ.get("PROMOTION_REQUIRE_HEALTH") == "1":
            health_cmd = os.environ.get("PROMOTION_HEALTH_CMD", "")
            if not health_cmd:
                print(
                    "promotion_deploy: PROMOTION_REQUIRE_HEALTH=1 needs "
                    "PROMOTION_HEALTH_CMD; refusing",
                    file=sys.stderr,
                )
                return 1
            try:
                code = run_health_cmd(health_cmd)
            except (ValueError, RuntimeError) as e:
                print("promotion_deploy: " + str(e), file=sys.stderr)
                return 1
            if code != 0:
                print(
                    "promotion_deploy: health check failed (exit "
                    + str(code)
                    + "); refusing to promote",
                    file=sys.stderr,
                )
                return 1
        promotion = build_promotion(
            artifact,
            outdir,
            DEPLOY_NAME,
            FROM_ENV,
            TO_ENV,
            VERSION,
            secret_refs,
        )
        print(
            "promotion_deploy: promoted "
            + FROM_ENV
            + " -> "
            + TO_ENV
            + " version "
            + VERSION
            + " (secrets by reference only, registry auth env-only)"
        )
        print("  promotion: " + promotion)
        return 0

    if os.environ.get("PROMOTION_REQUIRE_HEALTH") == "1" or os.environ.get(
        "PROMOTION_REGISTRY_REQUIRED"
    ) == "1":
        print(
            "promotion_deploy: local default stages only; health and registry gates "
            "apply to the live path (PROMOTION_LIVE=1); staged without running them",
            file=sys.stderr,
        )
    promotion = build_promotion(
        artifact,
        outdir,
        DEPLOY_NAME,
        FROM_ENV,
        TO_ENV,
        VERSION,
        secret_refs,
    )
    print(
        "promotion_deploy: staged "
        + FROM_ENV
        + " -> "
        + TO_ENV
        + " version "
        + VERSION
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  promotion: " + promotion)
    print("  manifest: " + os.path.join(promotion, "would-run.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
