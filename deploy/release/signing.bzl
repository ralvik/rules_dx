"""Signing + attestation selection for releases (issue #311).

Selected stack (signing-first per issue #26): Sigstore keyless
(`cosign sign-blob --bundle`, Fulcio OIDC + Rekor public-good on the
TUF trust root `https://tuf-repo-cdn.sigstore.dev`) plus GitHub Artifact
Attestations (`gh attestation create` / `gh attestation verify`). The
`cosign` and `gh` CLIs are host tools resolved at run time (like `tar`
for `archive_release`); no registry, no new module dependencies.

Safety (issue #5): signing never runs on CI push/PR. The deploy program
`sign_deploy.sh` prints the would-run commands with
`RELEASE_SIGN_DRY_RUN=1` (what CI exercises, publishes nothing) and
requires explicit owner approval plus OIDC identity for real signing
(see `docs/deploy/release-runbook.md`). Verification of the produced
bundles stays in `//deploy/install:dx_verify` (bundle-required, no
checksum-only fallback, fail-before-install).

Contract: `docs/deploy/release-runbook.md`.
"""

load("//deploy/rules:defs.bzl", "dx_deployment")

# Trust root is documented, not self-hosted.
SIGNING_TRUST_ROOT = "https://tuf-repo-cdn.sigstore.dev"
SIGNING_ISSUER = "https://token.actions.githubusercontent.com"

def signing_identity_error(identity, issuer):
    """Validates the expected certificate identity + issuer.

    Args:
      identity: candidate certificate identity (workflow identity).
      issuer: candidate OIDC issuer.

    Returns:
      "" when valid, else the failure reason.
    """
    if type(identity) != "string" or identity == "":
        return ("signing: invalid identity '" + str(identity) +
                "': want the owner-approved release workflow identity")
    if issuer != SIGNING_ISSUER:
        return ("signing: invalid issuer '" + str(issuer) +
                "': want '" + SIGNING_ISSUER + "' (Sigstore keyless via GitHub OIDC)")
    return ""

def signing_bundle_names(name):
    """Returns the deterministic bundle output names.

    Args:
      name: the signing instance name.

    Returns:
      A `(bundle, attestation-note)` tuple; bundles land next to the
      artifact as `<artifact>.bundle` at sign time (human-run), so this
      returns the launcher name only.
    """
    return (name + ".bundle", name + ".attestation")

def _signing_rlocation(ctx, f):
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp

def _signing_program_impl(ctx):
    asset_files = []
    asset_rlocs = []
    runfiles = ctx.runfiles()
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
        asset_rlocs.append(_signing_rlocation(ctx, f))
        runfiles = runfiles.merge(info.default_runfiles)
    deploy_file = ctx.file.deploy_sh
    deploy_rloc = _signing_rlocation(ctx, deploy_file)
    runfiles = runfiles.merge(ctx.runfiles(files = asset_files + [deploy_file]))
    asset_lines = "".join(["  \"$(rloc \"%s\")\"\n" % rloc for rloc in asset_rlocs])
    launcher = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `signed_release` (issue #311). Generated. Do not edit.
set -euo pipefail
if [[ -n "${RUNFILES_DIR:-}" && -d "${RUNFILES_DIR}" ]]; then
  RF="${RUNFILES_DIR}"
  MANIFEST=0
elif [[ -d "$0.runfiles" ]]; then
  RF="$0.runfiles"
  MANIFEST=0
elif [[ -f "$0.runfiles_manifest" ]]; then
  MANIFEST_FILE="$0.runfiles_manifest"
  MANIFEST=1
else
  echo "signing: cannot locate runfiles (tried RUNFILES_DIR, $0.runfiles)" >&2
  exit 1
fi
rloc() {
  local p="$1"
  if [[ "${MANIFEST}" == 1 ]]; then
    grep -sm1 "^${p} " "${MANIFEST_FILE}" | cut -f2- -d' '
  else
    printf "%%s/%%s" "${RF}" "${p}"
  fi
}
if [[ "$#" -gt 0 ]]; then
  echo "signing: this deploy target takes no extra args; artifacts are pinned at analysis time" >&2
  exit 1
fi
DEPLOY="$(rloc "%s")"
IDENTITY="%s"
ISSUER="%s"
ASSETS=(
%s)
export SIGNING_IDENTITY="${IDENTITY}"
export SIGNING_ISSUER="${ISSUER}"
exec "${DEPLOY}" "${ASSETS[@]}"
""" % (deploy_rloc, ctx.attr.identity, ctx.attr.issuer, asset_lines),
        is_executable = True,
    )
    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

_signing_program = rule(
    implementation = _signing_program_impl,
    executable = True,
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

    Creates `<name>_program` (runfiles-resolved launcher) and `<name>`
    (deployment returning `DxDeployInfo`). Run with
    `RELEASE_SIGN_DRY_RUN=1 bazel run :<name>` to print the would-run
    `cosign sign-blob` + `gh attestation` commands (what CI exercises,
    publishes nothing). Real signing needs the tag pushed beforehand,
    explicit owner approval, and OIDC identity per the runbook.

    Args:
      name: instance name; also the deploy target name.
      artifacts: labels of the files to sign.
      identity: expected certificate identity (release workflow id).
      issuer: OIDC issuer (default Sigstore keyless via GitHub OIDC).
      profile: default profile.
    """
    err = signing_identity_error(identity, issuer)
    if err != "":
        fail(err + " (in " + native.package_name() + ":" + name + ")")
    if len(artifacts) == 0:
        fail("signed_release " + native.package_name() + ":" + name + ": need at least one artifact")
    program_target = name + "_program"
    _signing_program(
        name = program_target,
        artifacts = artifacts,
        identity = identity,
        issuer = issuer,
    )
    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
