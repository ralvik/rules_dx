"""Credential-free release archives for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

def _archive_stage_impl(ctx):
    """Stages one executable as a single file preserving its basename.

    `sh_binary` (and other executable wrappers) expose more than one file
    via `DefaultInfo.files`, so `$(location :app)` fails with "expands to
    more than one file". The stage resolves `files_to_run.executable`
    once and symlinks it to `<stage>/<basename>`: the basename stays the
    original executable name (the tar member), while the parent directory
    keeps each `archive_deploy` instance distinct."""
    exe = ctx.attr.app[DefaultInfo].files_to_run.executable
    if exe == None:
        fail("archive_deploy " + str(ctx.label) + ": app " +
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
    doc = "Stages one executable for archive_deploy (single file, basename preserved).",
)

def _archive_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one release.

    The rule computes the runfiles rlocations for the staged app,
    tarball, and checksum via `rlocation_path`, then expands the shared
    `archive_deploy.py` template with those pins. The wrapping
    `py_binary` (see `archive_deploy`) carries the pinned inputs in
    `data` plus the Python runfiles library, so the launcher works under
    `bazel run`, `dx deploy` (which symlinks the `py_binary` entrypoint
    and merges its runfiles), and direct `bazel-bin` execution. Extra
    user args after `--` select the output directory (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    app_files = ctx.attr.app[DefaultInfo].files.to_list()
    if len(app_files) != 1:
        fail("archive_deploy " + str(ctx.label) + ": stage must provide exactly one file")
    archive_files = ctx.attr.archive[DefaultInfo].files.to_list()
    if len(archive_files) != 1:
        fail("archive_deploy " + str(ctx.label) + ": archive must provide exactly one file")
    checksum_files = ctx.attr.checksum[DefaultInfo].files.to_list()
    if len(checksum_files) != 1:
        fail("archive_deploy " + str(ctx.label) + ": checksum must provide exactly one file")
    app_file = app_files[0]
    archive_file = archive_files[0]
    checksum_file = checksum_files[0]

    app_rloc = rlocation_path(ctx, app_file)
    archive_rloc = rlocation_path(ctx, archive_file)
    checksum_rloc = rlocation_path(ctx, checksum_file)

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        # buildifier: disable=canonical-repository  # @@KEY@@ are template placeholders, not repo names
        substitutions = {
            "@@APP_RLOC@@": app_rloc,
            "@@CHECKSUM_RLOC@@": checksum_rloc,
            "@@TARBALL_RLOC@@": archive_rloc,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_archive_launcher = rule(
    implementation = _archive_launcher_impl,
    attrs = {
        "app": attr.label(mandatory = True),
        "archive": attr.label(mandatory = True),
        "checksum": attr.label(mandatory = True),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:archive_deploy.py",
        ),
    },
    doc = "Launcher template expansion for archive_deploy (wrapped as py_binary).",
)

def archive_filenames(name):
    """Returns the deterministic (tarball, checksum) output names."""
    return (name + ".tar.gz", name + ".tar.gz.sha256")

def archive_deploy(name, app, profile = "release"):
    """Packages one executable as a tarball + sha256 deployable target.

    Creates `<name>_stage` (single-file executable stage),
    `<name>_archive` (deterministic tarball via the hermetic
    `//deploy/rules:archiver` tool), `<name>_checksum` (sha256 via the
    hermetic `//deploy/rules:hasher` tool), `<name>_program_launcher`
    (expanded Python launcher resolving inputs via the Python runfiles
    library), `<name>_program` (`py_binary` on the managed Python 3.12
    toolchain wrapping the launcher with pinned `data` plus the runfiles
    library), and `<name>` (the `dx_deployment` returning
    `DxDeployInfo` with `app` and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; pass an output directory
    after `--` to choose where the artifacts land (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd). Deploy runtime is
    hermetic Python only (hashlib plus file copies): no bash, no host
    `tar`/`sha256sum`, no `sh_binary`.

    Args:
      name: Deploy target base name; derives stage/archive/program targets.
      app: Executable app target to package.
      profile: Deploy profile (debug, dev, or release).
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
    # The hermetic archiver dereferences the stage symlink (like
    # `tar -h`) and writes deterministic bytes (mtime 0, uid/gid 0,
    # gzip mtime 0); the hermetic hasher writes a sha256sum-compatible
    # line. Both run as declared genrule `tools` from the Rust
    # toolchain: no host `tar`/`sha256sum`/`shasum`, no
    # `command -v` probing, no shell-`$` escaping.
    native.genrule(
        name = archive_target,
        srcs = [":" + stage_target],
        outs = [tarball],
        tools = ["//deploy/rules:archiver"],
        cmd = "$(location //deploy/rules:archiver) $(location :" + stage_target + ") $(OUTS)",
    )

    native.genrule(
        name = checksum_target,
        srcs = [":" + archive_target],
        outs = [checksum],
        tools = ["//deploy/rules:hasher"],
        cmd = "$(location //deploy/rules:hasher) $(location :" + archive_target + ") $(OUTS)",
    )

    launcher_target = program_target + "_launcher"
    _archive_launcher(
        name = launcher_target,
        app = ":" + stage_target,
        archive = ":" + archive_target,
        checksum = ":" + checksum_target,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = [
            ":" + stage_target,
            ":" + archive_target,
            ":" + checksum_target,
        ],
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        app = app,
        deploy = ":" + program_target,
        profile = profile,
    )

def archive_release(name, app, profile = "release"):
    """Compat alias for `archive_deploy`.

    Kept for one release cycle, then removed.

    Args:
      name: Deploy target base name.
      app: Executable app target to package.
      profile: Deploy profile (debug, dev, or release).
    """
    archive_deploy(
        name = name,
        app = app,
        profile = profile,
    )
