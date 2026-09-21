"""BCR submission tooling for `rules_dx`.

Contract: `docs/deploy/release-runbook.md`.
"""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("//deploy/rules:defs.bzl", "dx_deployment")
load("//deploy/rules:launcher.bzl", "RUNFILES_BASH_INIT", "rlocation_path")

def bcr_source_error(module_name, version):
    """Validates the BCR module name + version pair."""
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
    """Validates whether a BCR submission may proceed."""
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
# Deploy launcher for `bcr_check`. Generated. Do not edit.
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
    construction."""
    src_err = bcr_source_error(module_name, version)
    if src_err != "":
        fail(src_err + " (in " + native.package_name() + ":" + name + ")")

    # BCR source template: archive `source.json` shape (URL + integrity
    # filled at release time by the human-run path; strip_url_prefix
    # follows the BCR publish layout). Deterministic, no network, no
    # host tools (Rust via declared `tools`).
    native.genrule(
        name = name + "_source",
        outs = [name + ".source.json"],
        tools = ["//deploy/release:bcr_source_gen"],
        cmd = "$(location //deploy/release:bcr_source_gen) $(OUTS) \"" + module_name + "\" \"" + version + "\"",
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
