"""BCR submission tooling for `rules_dx` (issue #311).

The approved v1 destinations (docs/environments/environment.md) are the
Bazel Central Registry for the module plus GitHub Releases for
standalone binaries. This macro implements the BCR half as an
owner-gated, dry-run-first check: it validates the module shape
(`rules_dx` name, SemVer version, no unpublishable `0.0.0` on real
submissions) and stages the `source.json` + integrity-hash + presubmit
inputs a BCR PR needs, without submitting anything.

Safety (issue #5): the deploy program `bcr_deploy.sh` prints the
would-submit PR with `BCR_DRY_RUN=1` (what CI exercises, submits
nothing) and requires explicit owner approval plus a real version for
any submission. The module stays at `0.0.0` until owners approve the
first release; `0.0.0` submissions fail analysis by construction.

Contract: `docs/deploy/release-runbook.md`.
"""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("//deploy/rules:defs.bzl", "dx_deployment")
load("//deploy/rules:launcher.bzl", "RUNFILES_BASH_INIT", "rlocation_path")

def bcr_source_error(module_name, version):
    """Validates the BCR module name + version pair.

    Args:
      module_name: candidate module name.
      version: candidate version string.

    Returns:
      "" when valid for a dry-run shape check, else the failure reason.
      `0.0.0` is valid for shape checks only (never submitted).
    """
    if module_name != "rules_dx":
        return ("bcr: invalid module '" + str(module_name) +
                "': want 'rules_dx'")
    if type(version) != "string" or version == "":
        return ("bcr: invalid version '" + str(version) +
                "': want SemVer MAJOR.MINOR.PATCH (for example '1.0.0')")
    parts = version.split(".")
    if len(parts) != 3:
        return ("bcr: invalid version '" + version +
                "': want SemVer MAJOR.MINOR.PATCH (for example '1.0.0')")
    for part in parts:
        if part == "" or not part[0].isdigit():
            return ("bcr: invalid version '" + version +
                    "': want SemVer MAJOR.MINOR.PATCH (for example '1.0.0')")
    return ""

def bcr_submit_error(version, approve):
    """Validates whether a BCR submission may proceed.

    Args:
      version: candidate version string.
      approve: owner approval flag (must be True for real submission).

    Returns:
      "" when submission may proceed, else the failure reason. `0.0.0`
      and unapproved submissions never proceed.
    """
    if version == "0.0.0":
        return ("bcr: version 0.0.0 is unpublishable (shape check only); " +
                "a real submission needs an owner-approved SemVer release version")
    if approve != True:
        return ("bcr: submission needs explicit owner approval per issue #5; " +
                "run with BCR_DRY_RUN=1 to print the would-submit PR")
    return ""

def _bcr_launcher_impl(ctx):
    """Writes the owner-gated BCR deploy launcher script.

    Resolves the source.json template + integrity file from runfiles via
    the standard `runfiles.bash` `rlocation` and execs `bcr_deploy.sh`
    with module/version. Extra user args are rejected: a submission is
    exactly the pinned inputs. All interpolations use `shell.quote`;
    wrapped as `sh_binary` (see `bcr_check`).
    """
    files = []
    rlocs = []
    for target in ctx.attr.inputs:
        info = target[DefaultInfo]
        fl = info.files.to_list()
        if len(fl) != 1:
            fail("bcr_check " + str(ctx.label) + ": input " +
                 str(target.label) + " provides " + str(len(fl)) +
                 " files, want exactly one")
        f = fl[0]
        files.append(f)
        rlocs.append(rlocation_path(ctx, f))
    deploy_file = ctx.file.deploy_sh
    deploy_rloc = rlocation_path(ctx, deploy_file)
    input_lines = "".join(["  \"$(rlocation " + shell.quote(r) + ")\"\n" for r in rlocs])
    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `bcr_check` (issue #311). Generated. Do not edit.
# Resolves inputs via the standard `runfiles.bash` `rlocation`; wrapped
# as `sh_binary` (see `bcr_check`).
set -euo pipefail
""" + RUNFILES_BASH_INIT + """if [[ "$#" -gt 0 ]]; then
  echo "bcr: this deploy target takes no extra args; the submission is exactly the pinned inputs" >&2
  exit 1
fi
DEPLOY="$(rlocation """ + shell.quote(deploy_rloc) + """)"
MODULE=""" + shell.quote(ctx.attr.module_name) + """
VERSION=""" + shell.quote(ctx.attr.version) + """
INPUTS=(
""" + input_lines + """)
exec "${DEPLOY}" "${MODULE}" "${VERSION}" "${INPUTS[@]}"
""",
        is_executable = True,
    )
    return [DefaultInfo(files = depset([launcher]))]

_bcr_launcher = rule(
    implementation = _bcr_launcher_impl,
    attrs = {
        "inputs": attr.label_list(mandatory = True),
        "module_name": attr.string(mandatory = True),
        "version": attr.string(mandatory = True),
        "deploy_sh": attr.label(
            allow_single_file = True,
            default = "//deploy/release:bcr_deploy.sh",
        ),
    },
)

def bcr_check(name, module_name = "rules_dx", version = "0.0.0", inputs = [], profile = "release"):
    """Creates an owner-gated BCR shape-check deploy target.

    Creates `<name>_source.json` (BCR source template for the version),
    `<name>_launcher` (generated launcher script via `runfiles.bash`
    `rlocation` with `shell.quote`), `<name>_program` (`sh_binary`
    wrapping the launcher with pinned `data` plus the runfiles library),
    and `<name>` (the `dx_deployment`). Run with `BCR_DRY_RUN=1 bazel run
    :<name>` to print the would-submit PR (what CI exercises, submits
    nothing). A real submission needs an owner-approved SemVer version
    plus explicit approval per the runbook; `0.0.0` fails submission by
    construction.

    Args:
      name: instance name; also the deploy target name.
      module_name: must be `rules_dx`.
      version: SemVer version (`0.0.0` = shape check only).
      inputs: extra pinned input files (integrity hash, presubmit).
      profile: default profile.
    """
    src_err = bcr_source_error(module_name, version)
    if src_err != "":
        fail(src_err + " (in " + native.package_name() + ":" + name + ")")

    # BCR source template: archive `source.json` shape (URL + integrity
    # filled at release time by the human-run path; strip_url_prefix
    # follows the BCR publish layout). Deterministic, no network.
    native.genrule(
        name = name + "_source",
        outs = [name + ".source.json"],
        cmd = "python3 - \"$(OUTS)\" \"" + module_name + "\" \"" + version + "\" <<'EOF'\n" +
              "import json,sys\n" +
              "out,mod,ver = sys.argv[1:4]\n" +
              "src={\"url\":\"https://github.com/ralvik/rules_dx/releases/download/v\"+ver+\"/\"+mod+\"-\"+ver+\".tar.gz\",\"integrity\":\"<integrity-filled-at-release>\",\"strip_prefix\":mod+\"-\"+ver}\n" +
              "open(out,\"w\",encoding=\"utf-8\").write(json.dumps(src,indent=2,sort_keys=True)+\"\\n\")\n" +
              "EOF",
    )

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    all_inputs = [":" + name + "_source"] + inputs
    _bcr_launcher(
        name = launcher_target,
        inputs = all_inputs,
        module_name = module_name,
        version = version,
    )

    sh_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = all_inputs + ["//deploy/release:bcr_deploy.sh"],
        deps = ["@rules_shell//shell/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
