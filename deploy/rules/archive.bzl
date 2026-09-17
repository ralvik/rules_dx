"""Credential-free release archives for `dx deploy` (issue #181).

`archive_release` is the first deploy macro: it proves the `DxDeployInfo`
-> `dx deploy` pattern with zero new pins. Only native `genrule`, two
small Starlark rules, and `dx_deployment` (`:defs.bzl`) participate. The
archive and checksum are built with host shell tools only (`tar`,
`sha256sum` with a `shasum -a 256` fallback for macOS); no registry, no
credentials, no new module dependencies.

Contract: `docs/deploy/authoring.md`. Deploy targets live next to the
app they release (for example `//rust/hello:release`).
"""

load(":defs.bzl", "dx_deployment")

def _archive_stage_impl(ctx):
    """Stages one executable as a single file preserving its basename.

    `sh_binary` (and other executable wrappers) expose more than one file
    via `DefaultInfo.files`, so `$(location :app)` fails with "expands to
    more than one file". The stage resolves `files_to_run.executable`
    once and symlinks it to `<stage>/<basename>`: the basename stays the
    original executable name (the tar member), while the parent directory
    keeps each `archive_release` instance distinct.

    Args:
      ctx: rule context with `app`.

    Returns:
      `DefaultInfo` with the single staged file.
    """
    exe = ctx.attr.app[DefaultInfo].files_to_run.executable
    if exe == None:
        fail("archive_release " + str(ctx.label) + ": app " +
             str(ctx.attr.app.label) + " has no executable")
    staged = ctx.actions.declare_file(ctx.label.name + "/" + exe.basename)
    ctx.actions.symlink(output = staged, target_file = exe)
    return [DefaultInfo(files = depset([staged]))]

_archive_stage = rule(
    implementation = _archive_stage_impl,
    attrs = {
        "app": attr.label(
            doc = "Executable to stage.",
            mandatory = True,
        ),
    },
    doc = "Stages one executable for archive_release (single file, basename preserved).",
)

def _archive_rlocation(ctx, f):
    """Returns the runfiles rlocation for one file.

    Args:
      ctx: rule context for the workspace name.
      f: the File to locate.

    Returns:
      The `workspace/short_path` rlocation string.
    """
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp

def _archive_program_impl(ctx):
    """Writes the executable deploy launcher for one release.

    `sh_binary(args=...)` cannot carry the tarball/checksum paths through
    `dx_deployment` (which symlinks only `files_to_run.executable`,
    dropping the target's `args`). The launcher instead resolves its
    three inputs from its own runfiles forest and execs
    `archive_deploy.sh` with them plus any user args (`"$@"` selects the
    output directory). Works under `bazel run`, `dx deploy`, and direct
    `bazel-bin` execution via `RUNFILES_DIR` / `$0.runfiles` / manifest.

    Args:
      ctx: rule context with `app`, `archive`, `checksum`, `deploy_sh`.

    Returns:
      `DefaultInfo` with the executable launcher and its runfiles.
    """
    app_files = ctx.attr.app[DefaultInfo].files.to_list()
    if len(app_files) != 1:
        fail("archive_release " + str(ctx.label) + ": stage must provide exactly one file")
    archive_files = ctx.attr.archive[DefaultInfo].files.to_list()
    if len(archive_files) != 1:
        fail("archive_release " + str(ctx.label) + ": archive must provide exactly one file")
    checksum_files = ctx.attr.checksum[DefaultInfo].files.to_list()
    if len(checksum_files) != 1:
        fail("archive_release " + str(ctx.label) + ": checksum must provide exactly one file")
    app_file = app_files[0]
    archive_file = archive_files[0]
    checksum_file = checksum_files[0]
    deploy_file = ctx.file.deploy_sh

    app_rloc = _archive_rlocation(ctx, app_file)
    archive_rloc = _archive_rlocation(ctx, archive_file)
    checksum_rloc = _archive_rlocation(ctx, checksum_file)
    deploy_rloc = _archive_rlocation(ctx, deploy_file)

    launcher = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `archive_release` (issue #181). Generated. Do not edit.
# Resolves the staged app, tarball, checksum, and deploy script from this
# launcher's runfiles forest, then execs the deploy script with them plus
# user args. `dx_deployment` symlinks only the executable, so `sh_binary`
# `args` cannot survive; runfiles lookup keeps the paths intact.
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
  echo "archive: cannot locate runfiles (tried RUNFILES_DIR, $0.runfiles)" >&2
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
DEPLOY="$(rloc "%s")"
APP="$(rloc "%s")"
TARBALL="$(rloc "%s")"
CHECKSUM="$(rloc "%s")"
exec "${DEPLOY}" "${APP}" "${TARBALL}" "${CHECKSUM}" "$@"
""" % (deploy_rloc, app_rloc, archive_rloc, checksum_rloc),
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = [app_file, archive_file, checksum_file, deploy_file])
    for target in [ctx.attr.app, ctx.attr.archive, ctx.attr.checksum]:
        runfiles = runfiles.merge(target[DefaultInfo].default_runfiles)
    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

_archive_program = rule(
    implementation = _archive_program_impl,
    executable = True,
    attrs = {
        "app": attr.label(mandatory = True),
        "archive": attr.label(mandatory = True),
        "checksum": attr.label(mandatory = True),
        "deploy_sh": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:archive_deploy.sh",
        ),
    },
    doc = "Executable deploy launcher for archive_release (runfiles-resolved).",
)

def archive_filenames(name):
    """Returns the deterministic (tarball, checksum) output names.

    Args:
      name: the `archive_release` instance name.

    Returns:
      A `(tarball, checksum)` string tuple, for example
      `("release.tar.gz", "release.tar.gz.sha256")`.
    """
    return (name + ".tar.gz", name + ".tar.gz.sha256")

def archive_release(name, app, profile = "release"):
    """Packages one executable as a tarball + sha256 deployable target.

    Creates `<name>_stage` (single-file executable stage),
    `<name>_archive` (genrule tarball via host `tar`),
    `<name>_checksum` (genrule sha256 via host `sha256sum`/`shasum`),
    `<name>_program` (runfiles-resolved deploy launcher), and `<name>`
    (the `dx_deployment` returning `DxDeployInfo` with `app` and
    `profile`). Run with `bazel run :<name>` or `dx deploy :<name>`;
    pass an output directory after `--` to choose where the artifacts
    land (default: `$BUILD_WORKSPACE_DIRECTORY`, else the cwd).

    Args:
      name: instance name; also the deploy target name.
      app: label of the executable to release.
      profile: default profile (`debug`, `dev`, or `release`).
    """
    (tarball, checksum) = archive_filenames(name)
    archive_target = name + "_archive"
    checksum_target = name + "_checksum"
    program_target = name + "_program"
    stage_target = name + "_stage"

    _archive_stage(
        name = stage_target,
        app = app,
    )

    # The stage is a single file (`<stage>/<exe basename>`), so
    # `$(location :stage)` is unambiguous for any executable kind
    # (`sh_binary` exposes two files and breaks `$(location :app)`).
    # `tar -h` dereferences the stage symlink so the archive holds the
    # binary bytes under the original basename, not a symlink.
    # genrule `cmd` undergoes Make expansion: `$(location ...)` and
    # `$(OUTS)` stay single-`$`, while shell `$` is escaped as `$$`.
    native.genrule(
        name = archive_target,
        srcs = [":" + stage_target],
        outs = [tarball],
        cmd = "set -euo pipefail; " +
              "app=\"$(location :" + stage_target + ")\"; " +
              "out=\"$(OUTS)\"; " +
              "case \"$$app\" in */*) d=\"$${app%/*}\";; *) d=\".\";; esac; " +
              "b=\"$${app##*/}\"; " +
              "tar -czhf \"$$out\" -C \"$$d\" \"$$b\"",
    )

    native.genrule(
        name = checksum_target,
        srcs = [":" + archive_target],
        outs = [checksum],
        cmd = "set -euo pipefail; " +
              "src=\"$(location :" + archive_target + ")\"; " +
              "out=\"$(OUTS)\"; " +
              "if command -v sha256sum >/dev/null 2>&1; then " +
              "sha256sum \"$$src\" > \"$$out\"; " +
              "else shasum -a 256 \"$$src\" > \"$$out\"; fi",
    )

    _archive_program(
        name = program_target,
        app = ":" + stage_target,
        archive = ":" + archive_target,
        checksum = ":" + checksum_target,
    )

    dx_deployment(
        name = name,
        app = app,
        deploy = ":" + program_target,
        profile = profile,
    )
