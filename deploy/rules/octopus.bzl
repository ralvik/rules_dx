"""Octopus Deploy publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned project/channel charset schema. Consumers query via
OCTOPUS_SCHEMA_VERSION = 1

# Project and channel names embed directly in the generated Python
# launcher, so the charset is restricted to what is safe inside single
# quotes.
_VALID_PROJECT_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

_VALID_CHANNEL_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

# Release versions embed directly in the generated Python launcher, so
# the charset is restricted to semver-safe characters inside single
# quotes (`+` covers build metadata).
_VALID_VERSION_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-+"

# Environment and space names embed directly in the generated Python
# launcher, so the charset is restricted to what is safe inside single
# quotes.
_VALID_ENVIRONMENT_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

OCTOPUS_DEFAULT_URL = "https://octopus.example.invalid"

def octopus_project_charset():
    """Returns the launcher-safe project charset via registry query.

    Derived from `_VALID_PROJECT_CHARS`, never duplicated.
    """
    return _VALID_PROJECT_CHARS

def octopus_channel_charset():
    """Returns the launcher-safe channel charset via registry query.

    Derived from `_VALID_CHANNEL_CHARS`, never duplicated.
    """
    return _VALID_CHANNEL_CHARS

def octopus_version_charset():
    """Returns the launcher-safe release-version charset via registry query.

    Derived from `_VALID_VERSION_CHARS`, never duplicated.
    """
    return _VALID_VERSION_CHARS

def octopus_environment_charset():
    """Returns the launcher-safe environment charset via registry query.

    Derived from `_VALID_ENVIRONMENT_CHARS`, never duplicated.
    """
    return _VALID_ENVIRONMENT_CHARS

def octopus_schema_error():
    """Validates the versioned project/channel charset schema.

    Checks data shape without pinning exact contents: version is v1,
    each charset is non-empty with unique launcher-safe characters and
    never admits quotes, backslash, space, or newline so names embed
    safely in the deploy launcher.
    """
    if OCTOPUS_SCHEMA_VERSION != 1:
        return "octopus project: unsupported schema v" + str(OCTOPUS_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_PROJECT_CHARS) != "string" or _VALID_PROJECT_CHARS == "":
        return "octopus project: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_PROJECT_CHARS.elems():
        if c in seen:
            return "octopus project: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "octopus project: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_CHANNEL_CHARS) != "string" or _VALID_CHANNEL_CHARS == "":
        return "octopus channel: want a non-empty charset (schema v1)"
    seen_channel = {}
    for c in _VALID_CHANNEL_CHARS.elems():
        if c in seen_channel:
            return "octopus channel: duplicate charset char '" + c + "'"
        seen_channel[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "octopus channel: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_VERSION_CHARS) != "string" or _VALID_VERSION_CHARS == "":
        return "octopus version: want a non-empty charset (schema v1)"
    seen_version = {}
    for c in _VALID_VERSION_CHARS.elems():
        if c in seen_version:
            return "octopus version: duplicate charset char '" + c + "'"
        seen_version[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/"]:
            return "octopus version: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_ENVIRONMENT_CHARS) != "string" or _VALID_ENVIRONMENT_CHARS == "":
        return "octopus environment: want a non-empty charset (schema v1)"
    seen_env = {}
    for c in _VALID_ENVIRONMENT_CHARS.elems():
        if c in seen_env:
            return "octopus environment: duplicate charset char '" + c + "'"
        seen_env[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "octopus environment: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def octopus_project_error(project):
    """Validates one Octopus project name value."""
    if type(project) != "string" or project == "":
        return ("octopus_deploy: invalid project '" + str(project) +
                "': want a non-empty project (for example 'octopus_demo')")
    for c in project.elems():
        if c not in _VALID_PROJECT_CHARS:
            return ("octopus_deploy: invalid project '" + project +
                    "': want only [A-Za-z0-9._-] so the project embeds " +
                    "safely in the deploy launcher")
    return ""

def octopus_channel_error(channel):
    """Validates one Octopus channel name value."""
    if type(channel) != "string" or channel == "":
        return ("octopus_deploy: invalid channel '" + str(channel) +
                "': want a non-empty channel (for example 'Default')")
    for c in channel.elems():
        if c not in _VALID_CHANNEL_CHARS:
            return ("octopus_deploy: invalid channel '" + channel +
                    "': want only [A-Za-z0-9._-] so the channel embeds " +
                    "safely in the deploy launcher")
    return ""

def octopus_version_error(version):
    """Validates one Octopus release version value."""
    if type(version) != "string" or version == "":
        return ("octopus_deploy: invalid version '" + str(version) +
                "': want a non-empty version (for example '0.0.0')")
    for c in version.elems():
        if c not in _VALID_VERSION_CHARS:
            return ("octopus_deploy: invalid version '" + version +
                    "': want only [A-Za-z0-9._-+] so the version embeds " +
                    "safely in the deploy launcher")
    return ""

def octopus_environment_error(environment):
    """Validates one Octopus environment (`--deploy-to`) value."""
    if type(environment) != "string" or environment == "":
        return ("octopus_deploy: invalid environment '" + str(environment) +
                "': want a non-empty environment (for example 'Production')")
    for c in environment.elems():
        if c not in _VALID_ENVIRONMENT_CHARS:
            return ("octopus_deploy: invalid environment '" + environment +
                    "': want only [A-Za-z0-9._-] so the environment embeds " +
                    "safely in the deploy launcher")
    return ""

def octopus_space_error(space):
    """Validates one Octopus space name value (empty means the default space)."""
    if space == "":
        return ""
    if type(space) != "string":
        return ("octopus_deploy: invalid space '" + str(space) +
                "': want a space name (for example 'Default') or empty for the default space")
    for c in space.elems():
        if c not in _VALID_ENVIRONMENT_CHARS:
            return ("octopus_deploy: invalid space '" + space +
                    "': want only [A-Za-z0-9._-] so the space embeds " +
                    "safely in the deploy launcher")
    return ""

def octopus_url_error(octopus_url):
    """Validates one Octopus server URL value."""
    if type(octopus_url) != "string" or octopus_url == "":
        return ("octopus_deploy: invalid octopus_url '" + str(octopus_url) +
                "': want a non-empty https URL (for example '" +
                OCTOPUS_DEFAULT_URL + "')")
    if not octopus_url.startswith("https://"):
        return ("octopus_deploy: invalid octopus_url '" + octopus_url +
                "': want an https URL so pushes never go over plaintext")
    for c in octopus_url.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("octopus_deploy: invalid octopus_url '" + octopus_url +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the URL embeds safely in the deploy launcher")
    return ""

def octopus_package_error(filename):
    """Validates one Octopus package filename value."""
    if type(filename) != "string" or filename == "":
        return ("octopus_deploy: invalid package '" + str(filename) +
                "': want a non-empty .tar.gz filename from archive_deploy")
    if not filename.endswith(".tar.gz"):
        return ("octopus_deploy: invalid package '" + filename +
                "': want a filename ending in .tar.gz (archive_deploy output)")
    for c in filename.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("octopus_deploy: invalid package '" + filename +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the filename embeds safely in the deploy launcher")
    return ""

def _octopus_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one Octopus deployment.

    The package resolves to a single `archive_deploy` output file. The
    rule computes the runfiles rlocation for the package via
    `rlocation_path`, then expands the shared `octopus_deploy.py`
    template with that pin plus project, channel, version, deploy-to
    environments (comma-joined), space, server URL, and drop name. The
    wrapping `py_binary` (see `octopus_deploy`) carries the pinned input
    in `data` plus the Python runfiles library, so the program works
    under `bazel run`, `dx deploy` (which symlinks the entrypoint and
    merges its runfiles), and direct `bazel-bin` execution. Extra user
    args after `--` select the output directory (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    package_files = ctx.attr.package[DefaultInfo].files.to_list()
    if len(package_files) != 1:
        fail("octopus_deploy " + str(ctx.label) + ": package " +
             str(ctx.attr.package.label) + " provides " +
             str(len(package_files)) + " files, want exactly one .tar.gz " +
             "(archive_deploy output)")
    package_file = package_files[0]
    package_error = octopus_package_error(package_file.basename)
    if package_error != "":
        fail(package_error + " (in " + str(ctx.label) + ")")
    package_rloc = rlocation_path(ctx, package_file)

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        # buildifier: disable=canonical-repository  # @@KEY@@ are template placeholders, not repo names
        substitutions = {
            "@@CHANNEL@@": ctx.attr.channel,
            "@@DEPLOY_NAME@@": ctx.attr.drop_name,
            "@@DEPLOY_TO@@": ",".join(ctx.attr.deploy_to),
            "@@OCTOPUS_URL@@": ctx.attr.octopus_url,
            "@@PACKAGE_RLOC@@": package_rloc,
            "@@PROJECT@@": ctx.attr.project,
            "@@SPACE@@": ctx.attr.space,
            "@@VERSION@@": ctx.attr.version,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_octopus_launcher = rule(
    implementation = _octopus_launcher_impl,
    attrs = {
        "channel": attr.string(
            doc = "Octopus channel baked into the release manifest.",
            mandatory = True,
        ),
        "deploy_to": attr.string_list(
            doc = "Optional deploy-to environments for create-release.",
        ),
        "drop_name": attr.string(
            doc = "Deploy target name baked into the package-drop directory.",
            mandatory = True,
        ),
        "octopus_url": attr.string(
            doc = "Live Octopus server URL, used only with explicit env.",
            mandatory = True,
        ),
        "package": attr.label(
            allow_single_file = True,
            doc = "Archive package file pinned in the drop directory.",
            mandatory = True,
        ),
        "project": attr.string(
            doc = "Octopus project baked into the release manifest.",
            mandatory = True,
        ),
        "space": attr.string(
            doc = "Octopus space; empty means the default space.",
            mandatory = True,
        ),
        "version": attr.string(
            doc = "Octopus release version baked into the release manifest.",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:octopus_deploy.py",
        ),
    },
    doc = "Launcher template expansion for octopus_deploy (wrapped as py_binary).",
)

def octopus_deploy(name, package, project, channel = "Default", version = "0.0.0", deploy_to = [], space = "", octopus_url = OCTOPUS_DEFAULT_URL, profile = "release"):
    """Publishes one archive package as a local-first Octopus deployment.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program` (`py_binary`
    on the managed Python 3.12 toolchain wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the default builds a local
    package-drop directory (`<name>-drop/` holding the pinned
    `archive_deploy` `.tar.gz` plus `would-run.txt` with the
    `create-release --project --channel` manifest) and verifies bytes,
    publishing nothing. Live `push` plus `create-release` runs only with
    `OCTOPUS_PUBLISH_LIVE=1`, `OCTOPUS_URL`, `OCTOPUS_API_KEY`, and
    `OCTOPUS_PUBLISH_APPROVED=1` after explicit owner approval.
    """
    project_error = octopus_project_error(project)
    if project_error != "":
        fail(project_error + " (in " + native.package_name() + ":" + name + ")")
    channel_error = octopus_channel_error(channel)
    if channel_error != "":
        fail(channel_error + " (in " + native.package_name() + ":" + name + ")")
    version_error = octopus_version_error(version)
    if version_error != "":
        fail(version_error + " (in " + native.package_name() + ":" + name + ")")
    for env in deploy_to:
        env_error = octopus_environment_error(env)
        if env_error != "":
            fail(env_error + " (in " + native.package_name() + ":" + name + ")")
    space_error = octopus_space_error(space)
    if space_error != "":
        fail(space_error + " (in " + native.package_name() + ":" + name + ")")
    url_error = octopus_url_error(octopus_url)
    if url_error != "":
        fail(url_error + " (in " + native.package_name() + ":" + name + ")")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _octopus_launcher(
        name = launcher_target,
        channel = channel,
        deploy_to = deploy_to,
        drop_name = name,
        octopus_url = octopus_url,
        package = package,
        project = project,
        space = space,
        version = version,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = [package],
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
