"""Signing + attestation selection for releases (issue #311).

Contract: `docs/deploy/release-runbook.md`.
"""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("//deploy/rules:defs.bzl", "dx_deployment")
load("//deploy/rules:launcher.bzl", "RUNFILES_BASH_INIT", "rlocation_path")

# Trust root is documented, not self-hosted.
SIGNING_TRUST_ROOT = "https://tuf-repo-cdn.sigstore.dev"
SIGNING_ISSUER = "https://token.actions.githubusercontent.com"

def signing_identity_error(identity, issuer):
    """Validates the expected certificate identity + issuer."""
    if type(identity) != "string" or identity == "":
        return ("signing: invalid identity '" + str(identity) +
                "': want the owner-approved release workflow identity")
    if issuer != SIGNING_ISSUER:
        return ("signing: invalid issuer '" + str(issuer) +
                "': want '" + SIGNING_ISSUER + "' (Sigstore keyless via GitHub OIDC)")
    return ""

def signing_bundle_names(name):
    """Returns the deterministic bundle output names."""
    return (name + ".bundle", name + ".attestation")

def _signing_launcher_impl(ctx):
    asset_files = []
    asset_rlocs = []
    for target in ctx.attr.artifacts:
        info = target[DefaultInfo]
        f = info.files_to_run.executable
        if f == None:
            files = info.files.to_list()
            if len(files) != 1:
                fail("signed_release " + str(ctx.label) + ": artifact " +
                     str(target.label) + " provides " +
                     str(len(files)) + " files, want exactly one")
            f = files[0]
        asset_files.append(f)
        asset_rlocs.append(rlocation_path(ctx, f))
    deploy_file = ctx.file.deploy_sh
    deploy_rloc = rlocation_path(ctx, deploy_file)
    asset_lines = "".join(["  \"$(rlocation " + shell.quote(rloc) + ")\"\n" for rloc in asset_rlocs])
    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `signed_release` (issue #311). Generated. Do not edit.
# Resolves inputs via the standard `runfiles.bash` `rlocation`; wrapped
# as `sh_binary` (see `signed_release`).
set -euo pipefail
""" + RUNFILES_BASH_INIT + """if [[ "$#" -gt 0 ]]; then
  echo "signing: this deploy target takes no extra args; artifacts are pinned at analysis time" >&2
  exit 1
fi
DEPLOY="$(rlocation """ + shell.quote(deploy_rloc) + """)"
IDENTITY=""" + shell.quote(ctx.attr.identity) + """
ISSUER=""" + shell.quote(ctx.attr.issuer) + """
ASSETS=(
""" + asset_lines + """)
export SIGNING_IDENTITY="${IDENTITY}"
export SIGNING_ISSUER="${ISSUER}"
exec "${DEPLOY}" "${ASSETS[@]}"
""",
        is_executable = True,
    )
    return [DefaultInfo(files = depset([launcher]))]

_signing_launcher = rule(
    implementation = _signing_launcher_impl,
    attrs = {
        "artifacts": attr.label_list(mandatory = True),
        "identity": attr.string(mandatory = True),
        "issuer": attr.string(mandatory = True),
        "deploy_sh": attr.label(
            allow_single_file = True,
            default = "//deploy/release:sign_deploy.sh",
        ),
    },
)

def signed_release(name, artifacts, identity, issuer = "https://token.actions.githubusercontent.com", profile = "release"):
    """Creates an owner-gated signing deploy target for pinned artifacts.

    Creates `<name>_launcher` (generated launcher script via
    `runfiles.bash` `rlocation` with `shell.quote`), `<name>_program`
    (`sh_binary` wrapping the launcher with pinned `data` plus the
    runfiles library), and `<name>` (deployment returning `DxDeployInfo`).
    Run with `RELEASE_SIGN_DRY_RUN=1 bazel run :<name>` to print the
    would-run `cosign sign-blob` + `gh attestation` commands (what CI
    exercises, publishes nothing). Real signing needs the tag pushed
    beforehand, explicit owner approval, and OIDC identity per the runbook."""
    err = signing_identity_error(identity, issuer)
    if err != "":
        fail(err + " (in " + native.package_name() + ":" + name + ")")
    if len(artifacts) == 0:
        fail("signed_release " + native.package_name() + ":" + name + ": need at least one artifact")
    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _signing_launcher(
        name = launcher_target,
        artifacts = artifacts,
        identity = identity,
        issuer = issuer,
    )
    sh_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = artifacts + ["//deploy/release:sign_deploy.sh"],
        deps = ["@rules_shell//shell/runfiles"],
    )
    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
