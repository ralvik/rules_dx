"""Local-first PyPI publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned name-charset schema. Consumers query via
# `pypi_name_charset` and `pypi_name_error` instead of duplicating the
# charset, so any charset evolution edits this one data constant with
# schema review, never a parallel allowlist.
PYPI_SCHEMA_VERSION = 1

# Distribution names embed directly in the generated Python launcher, so
# the charset is restricted to what is safe inside single quotes.
_VALID_NAME_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

PYPI_DEFAULT_REPOSITORY_URL = "https://upload.pypi.org/legacy/"

def pypi_name_charset():
    """Returns the launcher-safe distribution-name charset via registry query.

    Derived from `_VALID_NAME_CHARS`, never duplicated.
    """
    return _VALID_NAME_CHARS

def pypi_schema_error():
    """Validates the versioned distribution-name charset schema.

    Checks data shape without pinning exact contents: version is v1, the
    charset is non-empty with unique launcher-safe characters and never
    admits quotes, backslash, space, or newline so names embed safely in
    the generated Python launcher.
    """
    if PYPI_SCHEMA_VERSION != 1:
        return "pypi name: unsupported schema v" + str(PYPI_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_NAME_CHARS) != "string" or _VALID_NAME_CHARS == "":
        return "pypi name: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_NAME_CHARS.elems():
        if c in seen:
            return "pypi name: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "pypi name: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def pypi_name_error(dist_name):
    """Validates one distribution name value."""
    if type(dist_name) != "string" or dist_name == "":
        return ("pypi_deploy: invalid distribution name '" + str(dist_name) +
                "': want a non-empty name (for example 'pypi_demo')")
    for c in dist_name.elems():
        if c not in _VALID_NAME_CHARS:
            return ("pypi_deploy: invalid distribution name '" + dist_name +
                    "': want only [A-Za-z0-9._-] so the name embeds " +
                    "safely in the deploy launcher")
    return ""

def pypi_repository_error(repository_url):
    """Validates one PyPI repository URL value."""
    if type(repository_url) != "string" or repository_url == "":
        return ("pypi_deploy: invalid repository_url '" + str(repository_url) +
                "': want a non-empty https URL (for example '" +
                PYPI_DEFAULT_REPOSITORY_URL + "')")
    if not repository_url.startswith("https://"):
        return ("pypi_deploy: invalid repository_url '" + repository_url +
                "': want an https URL so uploads never go over plaintext")
    for c in repository_url.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("pypi_deploy: invalid repository_url '" + repository_url +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the URL embeds safely in the deploy launcher")
    return ""

def pypi_wheel_error(filename):
    """Validates one wheel filename value."""
    if type(filename) != "string" or filename == "":
        return ("pypi_deploy: invalid wheel '" + str(filename) +
                "': want a non-empty .whl filename")
    if not filename.endswith(".whl"):
        return ("pypi_deploy: invalid wheel '" + filename +
                "': want a filename ending in .whl")
    for c in filename.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("pypi_deploy: invalid wheel '" + filename +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the filename embeds safely in the deploy launcher")
    return ""

def _pypi_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one PyPI deployment.

    Each distribution file resolves to a single file. The rule computes
    the runfiles rlocations for the wheel plus the optional sdist via
    `rlocation_path`, then expands the shared `pypi_deploy.py` template
    with those pins plus the distribution name and repository URL. The
    wrapping `py_binary` (see `pypi_deploy`) carries the pinned inputs
    in `data` plus the Python runfiles library, so the program works
    under `bazel run`, `dx deploy` (which symlinks the entrypoint and
    merges its runfiles), and direct `bazel-bin` execution. Extra user
    args after `--` select the output directory (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    wheel_files = ctx.attr.wheel[DefaultInfo].files.to_list()
    if len(wheel_files) != 1:
        fail("pypi_deploy " + str(ctx.label) + ": wheel " +
             str(ctx.attr.wheel.label) + " provides " +
             str(len(wheel_files)) + " files, want exactly one .whl")
    wheel_file = wheel_files[0]
    wheel_error = pypi_wheel_error(wheel_file.basename)
    if wheel_error != "":
        fail(wheel_error + " (in " + str(ctx.label) + ")")
    wheel_rloc = rlocation_path(ctx, wheel_file)

    sdist_rloc = ""
    if ctx.attr.sdist:
        sdist_files = ctx.attr.sdist[DefaultInfo].files.to_list()
        if len(sdist_files) != 1:
            fail("pypi_deploy " + str(ctx.label) + ": sdist " +
                 str(ctx.attr.sdist.label) + " provides " +
                 str(len(sdist_files)) + " files, want exactly one .tar.gz")
        sdist_file = sdist_files[0]
        if not sdist_file.basename.endswith(".tar.gz"):
            fail("pypi_deploy " + str(ctx.label) + ": sdist '" +
                 sdist_file.basename + "' must end in .tar.gz")
        sdist_rloc = rlocation_path(ctx, sdist_file)

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        # buildifier: disable=canonical-repository  # @@KEY@@ are template placeholders, not repo names
        substitutions = {
            "@@DIST_NAME@@": ctx.attr.dist_name,
            "@@REPOSITORY_URL@@": ctx.attr.repository_url,
            "@@SDIST_RLOC@@": sdist_rloc,
            "@@WHEEL_RLOC@@": wheel_rloc,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_pypi_launcher = rule(
    implementation = _pypi_launcher_impl,
    attrs = {
        "dist_name": attr.string(
            doc = "Distribution name baked into the wheelhouse directory.",
            mandatory = True,
        ),
        "repository_url": attr.string(
            doc = "Live-upload repository URL, used only with explicit env.",
            mandatory = True,
        ),
        "sdist": attr.label(
            allow_single_file = True,
            doc = "Optional sdist file pinned in the wheelhouse.",
        ),
        "wheel": attr.label(
            allow_single_file = True,
            doc = "Wheel file pinned in the wheelhouse.",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:pypi_deploy.py",
        ),
    },
    doc = "Launcher template expansion for pypi_deploy (wrapped as py_binary).",
)

def pypi_deploy(name, wheel, sdist = None, repository_url = PYPI_DEFAULT_REPOSITORY_URL, profile = "release"):
    """Publishes one wheel plus an optional sdist as a local-first PyPI deployment.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program` (`py_binary`
    on the managed Python 3.12 toolchain wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the default builds a local
    wheelhouse directory (`<dist>-wheelhouse/` with `simple-index/` plus
    the `.whl`) and verifies bytes, publishing nothing. Live
    `twine upload --non-interactive --repository-url` runs only with
    `PYPI_PUBLISH_LIVE=1`, `PYPI_API_TOKEN`, and `PYPI_PUBLISH_APPROVED=1`
    after explicit owner approval.
    """
    name_error = pypi_name_error(name)
    if name_error != "":
        fail(name_error + " (in " + native.package_name() + ":" + name + ")")
    repository_error = pypi_repository_error(repository_url)
    if repository_error != "":
        fail(repository_error + " (in " + native.package_name() + ":" + name + ")")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _pypi_launcher(
        name = launcher_target,
        dist_name = name,
        repository_url = repository_url,
        sdist = sdist,
        wheel = wheel,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    data = [wheel]
    if sdist != None:
        data.append(sdist)
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = data,
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
