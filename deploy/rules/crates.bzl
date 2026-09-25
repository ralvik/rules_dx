"""Local-first crates.io publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned name/version charset schema. Consumers query via
# `crates_name_charset`, `crates_version_charset`, and `crates_*_error`
# instead of duplicating the charset, so any charset evolution edits this
# one data constant with schema review, never a parallel allowlist.
CRATES_SCHEMA_VERSION = 1

# Crate names embed directly in the generated Python launcher, so the
# charset is restricted to what is safe inside single quotes. Dots stay
# out (crates.io names use `-`/`_` only).
_VALID_NAME_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-"

# Crate versions embed directly in the generated Python launcher, so the
# charset is restricted to semver-safe characters inside single quotes
# (`+` covers build metadata).
_VALID_VERSION_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-+"

def crates_name_charset():
    """Returns the launcher-safe crate-name charset via registry query.

    Derived from `_VALID_NAME_CHARS`, never duplicated.
    """
    return _VALID_NAME_CHARS

def crates_version_charset():
    """Returns the launcher-safe crate-version charset via registry query.

    Derived from `_VALID_VERSION_CHARS`, never duplicated.
    """
    return _VALID_VERSION_CHARS

def crates_schema_error():
    """Validates the versioned crate name/version charset schema.

    Checks data shape without pinning exact contents: version is v1, each
    charset is non-empty with unique launcher-safe characters and never
    admits quotes, backslash, space, or newline so names and versions
    embed safely in the deploy launcher.
    """
    if CRATES_SCHEMA_VERSION != 1:
        return "crates name: unsupported schema v" + str(CRATES_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_NAME_CHARS) != "string" or _VALID_NAME_CHARS == "":
        return "crates name: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_NAME_CHARS.elems():
        if c in seen:
            return "crates name: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+", "."]:
            return "crates name: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_VERSION_CHARS) != "string" or _VALID_VERSION_CHARS == "":
        return "crates version: want a non-empty charset (schema v1)"
    seen_version = {}
    for c in _VALID_VERSION_CHARS.elems():
        if c in seen_version:
            return "crates version: duplicate charset char '" + c + "'"
        seen_version[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/"]:
            return "crates version: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def crates_name_error(crate_name):
    """Validates one crate name value."""
    if type(crate_name) != "string" or crate_name == "":
        return ("crates_deploy: invalid crate name '" + str(crate_name) +
                "': want a non-empty name (for example 'crates_demo')")
    for c in crate_name.elems():
        if c not in _VALID_NAME_CHARS:
            return ("crates_deploy: invalid crate name '" + crate_name +
                    "': want only [A-Za-z0-9_-] so the name embeds " +
                    "safely in the deploy launcher")
    return ""

def crates_version_error(version):
    """Validates one crate version value."""
    if type(version) != "string" or version == "":
        return ("crates_deploy: invalid version '" + str(version) +
                "': want a non-empty version (for example '0.0.0')")
    for c in version.elems():
        if c not in _VALID_VERSION_CHARS:
            return ("crates_deploy: invalid version '" + version +
                    "': want only [A-Za-z0-9._-+] so the version embeds " +
                    "safely in the deploy launcher")
    return ""

def crates_file_error(filename):
    """Validates one crate source filename value."""
    if type(filename) != "string" or filename == "":
        return ("crates_deploy: invalid crate file '" + str(filename) +
                "': want a non-empty filename")
    for c in filename.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("crates_deploy: invalid crate file '" + filename +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the filename embeds safely in the deploy launcher")
    return ""

def crates_allow_dirty_error(allow_dirty):
    """Validates the clean-tree gate."""
    if allow_dirty != False:
        return ("crates_deploy: allow_dirty=True requires explicit owner " +
                "approval; keep a clean tree and publish from committed " +
                "sources instead")
    return ""

def _crates_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one crates.io deployment.

    Each crate source resolves to one or more files. The rule computes
    the runfiles rlocations for every pinned source via `rlocation_path`,
    joins them with `;`, then expands the shared `crates_deploy.py`
    template with those pins plus the crate name, version, and dirty gate.
    The wrapping `py_binary` (see `crates_deploy`) carries the pinned
    inputs in `data` plus the Python runfiles library, so the program
    works under `bazel run`, `dx deploy` (which symlinks the entrypoint
    and merges its runfiles), and direct `bazel-bin` execution. Extra user
    args after `--` select the output directory (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    crate_files = ctx.attr.crate[DefaultInfo].files.to_list()
    if len(crate_files) == 0:
        fail("crates_deploy " + str(ctx.label) + ": crate " +
             str(ctx.attr.crate.label) + " provides no files, want at least one source file")
    rlocs = []
    for f in crate_files:
        file_error = crates_file_error(f.basename)
        if file_error != "":
            fail(file_error + " (in " + str(ctx.label) + ")")
        rlocs.append(rlocation_path(ctx, f))

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        # buildifier: disable=canonical-repository  # @@KEY@@ are template placeholders, not repo names
        substitutions = {
            "@@ALLOW_DIRTY@@": "1" if ctx.attr.allow_dirty else "0",
            "@@CRATE_NAME@@": ctx.attr.crate_name,
            "@@CRATE_RLOCS@@": ";".join(rlocs),
            "@@CRATE_VERSION@@": ctx.attr.version,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_crates_launcher = rule(
    implementation = _crates_launcher_impl,
    attrs = {
        "allow_dirty": attr.bool(
            doc = "Dirty-tree gate baked into the launcher; must stay False.",
        ),
        "crate": attr.label(
            allow_files = True,
            doc = "Staged crate source files pinned in the vendor directory.",
            mandatory = True,
        ),
        "crate_name": attr.string(
            doc = "Crate name baked into the vendor directory.",
            mandatory = True,
        ),
        "version": attr.string(
            doc = "Crate version baked into the file registry.",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:crates_deploy.py",
        ),
    },
    doc = "Launcher template expansion for crates_deploy (wrapped as py_binary).",
)

def crates_deploy(name, crate, version = "0.0.0", allow_dirty = False, profile = "release"):
    """Publishes staged crate sources as a local-first crates.io deployment.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program` (`py_binary`
    on the managed Python 3.12 toolchain wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the default builds a local
    vendor directory (`<crate>-vendor/` with `vendor/` plus a file
    registry) and verifies bytes, publishing nothing. Live
    `cargo publish` runs only with `CRATES_PUBLISH_LIVE=1`,
    `CARGO_REGISTRY_TOKEN`, and `CRATES_PUBLISH_APPROVED=1` after explicit
    owner approval, and never passes `--allow-dirty`.
    """
    name_error = crates_name_error(name)
    if name_error != "":
        fail(name_error + " (in " + native.package_name() + ":" + name + ")")
    version_error = crates_version_error(version)
    if version_error != "":
        fail(version_error + " (in " + native.package_name() + ":" + name + ")")
    dirty_error = crates_allow_dirty_error(allow_dirty)
    if dirty_error != "":
        fail(dirty_error + " (in " + native.package_name() + ":" + name + ")")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _crates_launcher(
        name = launcher_target,
        allow_dirty = allow_dirty,
        crate = crate,
        crate_name = name,
        version = version,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = [crate],
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
