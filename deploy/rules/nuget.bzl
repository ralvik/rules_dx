"""Local-first NuGet publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned id/version charset schema. Consumers query via
# `nuget_id_charset`, `nuget_version_charset`, and `nuget_*_error`
# instead of duplicating the charset, so any charset evolution edits this
# one data constant with schema review, never a parallel allowlist.
NUGET_SCHEMA_VERSION = 1

# Package ids embed directly in the generated Python launcher, so the
# charset is restricted to what is safe inside single quotes.
_VALID_ID_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

# Package versions embed directly in the generated Python launcher, so the
# charset is restricted to semver-safe characters inside single quotes
# (`+` covers build metadata).
_VALID_VERSION_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-+"

NUGET_DEFAULT_SOURCE = "https://api.nuget.org/v3/index.json"

def nuget_id_charset():
    """Returns the launcher-safe package-id charset via registry query.

    Derived from `_VALID_ID_CHARS`, never duplicated.
    """
    return _VALID_ID_CHARS

def nuget_version_charset():
    """Returns the launcher-safe package-version charset via registry query.

    Derived from `_VALID_VERSION_CHARS`, never duplicated.
    """
    return _VALID_VERSION_CHARS

def nuget_schema_error():
    """Validates the versioned package id/version charset schema.

    Checks data shape without pinning exact contents: version is v1, each
    charset is non-empty with unique launcher-safe characters and never
    admits quotes, backslash, space, or newline so ids and versions
    embed safely in the deploy launcher."""
    if NUGET_SCHEMA_VERSION != 1:
        return "nuget id: unsupported schema v" + str(NUGET_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_ID_CHARS) != "string" or _VALID_ID_CHARS == "":
        return "nuget id: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_ID_CHARS.elems():
        if c in seen:
            return "nuget id: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "nuget id: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_VERSION_CHARS) != "string" or _VALID_VERSION_CHARS == "":
        return "nuget version: want a non-empty charset (schema v1)"
    seen_version = {}
    for c in _VALID_VERSION_CHARS.elems():
        if c in seen_version:
            return "nuget version: duplicate charset char '" + c + "'"
        seen_version[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/"]:
            return "nuget version: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def nuget_id_error(package_id):
    """Validates one package id value."""
    if type(package_id) != "string" or package_id == "":
        return ("nuget_deploy: invalid package id '" + str(package_id) +
                "': want a non-empty id (for example 'nuget_demo')")
    for c in package_id.elems():
        if c not in _VALID_ID_CHARS:
            return ("nuget_deploy: invalid package id '" + package_id +
                    "': want only [A-Za-z0-9._-] so the id embeds " +
                    "safely in the deploy launcher")
    return ""

def nuget_version_error(version):
    """Validates one package version value."""
    if type(version) != "string" or version == "":
        return ("nuget_deploy: invalid version '" + str(version) +
                "': want a non-empty version (for example '0.0.0')")
    for c in version.elems():
        if c not in _VALID_VERSION_CHARS:
            return ("nuget_deploy: invalid version '" + version +
                    "': want only [A-Za-z0-9._-+] so the version embeds " +
                    "safely in the deploy launcher")
    return ""

def nuget_nupkg_error(filename):
    """Validates one nupkg filename value."""
    if type(filename) != "string" or filename == "":
        return ("nuget_deploy: invalid nupkg '" + str(filename) +
                "': want a non-empty .nupkg filename")
    if not filename.endswith(".nupkg"):
        return ("nuget_deploy: invalid nupkg '" + filename +
                "': want a filename ending in .nupkg")
    for c in filename.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("nuget_deploy: invalid nupkg '" + filename +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the filename embeds safely in the deploy launcher")
    return ""

def nuget_source_error(source):
    """Validates one NuGet source URL value."""
    if type(source) != "string" or source == "":
        return ("nuget_deploy: invalid source '" + str(source) +
                "': want a non-empty https URL (for example '" +
                NUGET_DEFAULT_SOURCE + "')")
    if not source.startswith("https://"):
        return ("nuget_deploy: invalid source '" + source +
                "': want an https URL so pushes never go over plaintext")
    for c in source.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("nuget_deploy: invalid source '" + source +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the URL embeds safely in the deploy launcher")
    return ""

def _nuget_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one NuGet deployment.

    The package file resolves to a single file. The rule computes the
    runfiles rlocation for the nupkg via `rlocation_path`, then expands
    the shared `nuget_deploy.py` template with that pin plus the package
    id, version, and source. The wrapping `py_binary` (see `nuget_deploy`)
    carries the pinned input in `data` plus the Python runfiles library,
    so the program works under `bazel run`, `dx deploy` (which symlinks
    the entrypoint and merges its runfiles), and direct `bazel-bin`
    execution. Extra user args after `--` select the output directory
    (default: `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    nupkg_files = ctx.attr.nupkg[DefaultInfo].files.to_list()
    if len(nupkg_files) != 1:
        fail("nuget_deploy " + str(ctx.label) + ": nupkg " +
             str(ctx.attr.nupkg.label) + " provides " +
             str(len(nupkg_files)) + " files, want exactly one .nupkg")
    nupkg_file = nupkg_files[0]
    nupkg_error = nuget_nupkg_error(nupkg_file.basename)
    if nupkg_error != "":
        fail(nupkg_error + " (in " + str(ctx.label) + ")")
    nupkg_rloc = rlocation_path(ctx, nupkg_file)

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        substitutions = {
            "@@NUPKG_RLOC@@": nupkg_rloc,
            "@@PACKAGE_ID@@": ctx.attr.package_id,
            "@@PACKAGE_SOURCE@@": ctx.attr.source,
            "@@PACKAGE_VERSION@@": ctx.attr.version,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_nuget_launcher = rule(
    implementation = _nuget_launcher_impl,
    attrs = {
        "nupkg": attr.label(
            allow_single_file = True,
            doc = "Package file pinned in the folder feed.",
            mandatory = True,
        ),
        "package_id": attr.string(
            doc = "Package id baked into the folder feed directory.",
            mandatory = True,
        ),
        "source": attr.string(
            doc = "Live-push source URL, used only with explicit env.",
            mandatory = True,
        ),
        "version": attr.string(
            doc = "Package version baked into the launcher.",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:nuget_deploy.py",
        ),
    },
    doc = "Launcher template expansion for nuget_deploy (wrapped as py_binary).",
)

def nuget_deploy(name, nupkg, version = "0.0.0", source = NUGET_DEFAULT_SOURCE, profile = "release"):
    """Publishes one nupkg as a local-first NuGet deployment.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program` (`py_binary`
    on the managed Python 3.12 toolchain wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the default builds a local
    folder feed directory (`<id>-feed/` holding the pinned `.nupkg`,
    usable as a `dotnet` source) and verifies bytes, publishing nothing.
    Live `dotnet nuget push --source --api-key --skip-duplicate` runs only
    with `NUGET_PUBLISH_LIVE=1`, `NUGET_API_KEY`, and
    `NUGET_PUBLISH_APPROVED=1` after explicit owner approval."""
    name_error = nuget_id_error(name)
    if name_error != "":
        fail(name_error + " (in " + native.package_name() + ":" + name + ")")
    version_error = nuget_version_error(version)
    if version_error != "":
        fail(version_error + " (in " + native.package_name() + ":" + name + ")")
    source_error = nuget_source_error(source)
    if source_error != "":
        fail(source_error + " (in " + native.package_name() + ":" + name + ")")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _nuget_launcher(
        name = launcher_target,
        nupkg = nupkg,
        package_id = name,
        source = source,
        version = version,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = [nupkg],
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
