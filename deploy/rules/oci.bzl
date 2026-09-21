"""Local-first OCI publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned registry/tag charset schema. Consumers query via
# `oci_registry_charset`, `oci_tag_charset`, and `oci_*_error`
# instead of duplicating the charset, so any charset evolution edits this
# one data constant with schema review, never a parallel allowlist.
OCI_SCHEMA_VERSION = 1

# Image tags embed directly in the generated Python launcher, so the
# charset is restricted to what is safe inside single quotes.
_VALID_TAG_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

# Registries embed directly in the generated Python launcher, so the
# charset covers host plus optional repository path and port.
_VALID_REGISTRY_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-/:"

# Repositories embed directly in the generated Python launcher, so the
# charset covers path segments without ports.
_VALID_REPOSITORY_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-/"

OCI_DEFAULT_REGISTRY = "ghcr.io"

def oci_tag_charset():
    """Returns the launcher-safe image-tag charset via registry query.

    Derived from `_VALID_TAG_CHARS`, never duplicated.
    """
    return _VALID_TAG_CHARS

def oci_registry_charset():
    """Returns the launcher-safe registry charset via registry query.

    Derived from `_VALID_REGISTRY_CHARS`, never duplicated.
    """
    return _VALID_REGISTRY_CHARS

def oci_repository_charset():
    """Returns the launcher-safe repository charset via registry query.

    Derived from `_VALID_REPOSITORY_CHARS`, never duplicated.
    """
    return _VALID_REPOSITORY_CHARS

def oci_schema_error():
    """Validates the versioned registry/tag charset schema.

    Checks data shape without pinning exact contents: version is v1, each
    charset is non-empty with unique launcher-safe characters and never
    admits quotes, backslash, space, or newline so values embed safely
    in the deploy launcher."""
    if OCI_SCHEMA_VERSION != 1:
        return "oci tag: unsupported schema v" + str(OCI_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_TAG_CHARS) != "string" or _VALID_TAG_CHARS == "":
        return "oci tag: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_TAG_CHARS.elems():
        if c in seen:
            return "oci tag: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "oci tag: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_REGISTRY_CHARS) != "string" or _VALID_REGISTRY_CHARS == "":
        return "oci registry: want a non-empty charset (schema v1)"
    seen_registry = {}
    for c in _VALID_REGISTRY_CHARS.elems():
        if c in seen_registry:
            return "oci registry: duplicate charset char '" + c + "'"
        seen_registry[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ",", "+", "=", "%", "@"]:
            return "oci registry: unsafe charset char '" + c + "' (must stay launcher-safe)"
    if type(_VALID_REPOSITORY_CHARS) != "string" or _VALID_REPOSITORY_CHARS == "":
        return "oci repository: want a non-empty charset (schema v1)"
    seen_repo = {}
    for c in _VALID_REPOSITORY_CHARS.elems():
        if c in seen_repo:
            return "oci repository: duplicate charset char '" + c + "'"
        seen_repo[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "+", "=", "%", "@"]:
            return "oci repository: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def oci_tag_error(tag):
    """Validates one image tag value."""
    if type(tag) != "string" or tag == "":
        return ("oci_deploy: invalid tag '" + str(tag) +
                "': want a non-empty tag (for example 'latest')")
    for c in tag.elems():
        if c not in _VALID_TAG_CHARS:
            return ("oci_deploy: invalid tag '" + tag +
                    "': want only [A-Za-z0-9._-] so the tag embeds " +
                    "safely in the deploy launcher")
    return ""

def oci_registry_error(registry):
    """Validates one registry value."""
    if type(registry) != "string" or registry == "":
        return ("oci_deploy: invalid registry '" + str(registry) +
                "': want a non-empty registry (for example '" +
                OCI_DEFAULT_REGISTRY + "')")
    for c in registry.elems():
        if c not in _VALID_REGISTRY_CHARS:
            return ("oci_deploy: invalid registry '" + registry +
                    "': want only [A-Za-z0-9._-/:] so the registry embeds " +
                    "safely in the deploy launcher")
    if registry.startswith("/") or registry.endswith("/"):
        return ("oci_deploy: invalid registry '" + registry +
                "': want no leading or trailing slash")
    if "://" in registry:
        return ("oci_deploy: invalid registry '" + registry +
                "': want a host plus optional path, never a URL scheme")
    for c in registry.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("oci_deploy: invalid registry '" + registry +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the registry embeds safely in the deploy launcher")
    return ""

def oci_repository_error(repository):
    """Validates one repository value."""
    if type(repository) != "string" or repository == "":
        return ("oci_deploy: invalid repository '" + str(repository) +
                "': want a non-empty repository (for example 'oci_demo')")
    for c in repository.elems():
        if c not in _VALID_REPOSITORY_CHARS:
            return ("oci_deploy: invalid repository '" + repository +
                    "': want only [A-Za-z0-9._-/] so the repository embeds " +
                    "safely in the deploy launcher")
    if repository.startswith("/") or repository.endswith("/"):
        return ("oci_deploy: invalid repository '" + repository +
                "': want no leading or trailing slash")
    if "//" in repository:
        return ("oci_deploy: invalid repository '" + repository +
                "': want no empty path segments")
    return ""

def oci_tar_error(filename):
    """Validates one image-tar filename value."""
    if type(filename) != "string" or filename == "":
        return ("oci_deploy: invalid image_tar '" + str(filename) +
                "': want a non-empty .tar filename")
    if not (filename.endswith(".tar") or filename.endswith(".tar.gz")):
        return ("oci_deploy: invalid image_tar '" + filename +
                "': want a filename ending in .tar or .tar.gz")
    for c in filename.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("oci_deploy: invalid image_tar '" + filename +
                    "': want no quotes, backslashes, spaces, or newlines " +
                    "so the filename embeds safely in the deploy launcher")
    return ""

def _oci_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one OCI deployment.

    The image tar resolves to a single file. The rule computes the
    runfiles rlocation via `rlocation_path`, then expands the shared
    `oci_deploy.py` template with that pin plus registry, repository,
    and tag. The wrapping `py_binary` (see `oci_deploy`) carries the
    pinned input in `data` plus the Python runfiles library, so the
    program works under `bazel run`, `dx deploy`, and direct
    `bazel-bin` execution. Extra user args after `--` select the output
    directory (default: `$BUILD_WORKSPACE_DIRECTORY`, else the cwd)."""
    tar_files = ctx.attr.image_tar[DefaultInfo].files.to_list()
    if len(tar_files) != 1:
        fail("oci_deploy " + str(ctx.label) + ": image_tar " +
             str(ctx.attr.image_tar.label) + " provides " +
             str(len(tar_files)) + " files, want exactly one .tar")
    tar_file = tar_files[0]
    tar_error = oci_tar_error(tar_file.basename)
    if tar_error != "":
        fail(tar_error + " (in " + str(ctx.label) + ")")
    tar_rloc = rlocation_path(ctx, tar_file)

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        substitutions = {
            "@@IMAGE_RLOC@@": tar_rloc,
            "@@OCI_REGISTRY@@": ctx.attr.registry,
            "@@OCI_REPOSITORY@@": ctx.attr.repository,
            "@@OCI_TAG@@": ctx.attr.tag,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_oci_launcher = rule(
    implementation = _oci_launcher_impl,
    attrs = {
        "image_tar": attr.label(
            allow_single_file = True,
            doc = "Image tar file pinned in the OCI layout.",
            mandatory = True,
        ),
        "registry": attr.string(
            doc = "Registry host plus optional path, used only with explicit env.",
            mandatory = True,
        ),
        "repository": attr.string(
            doc = "Repository name baked into the layout annotation.",
            mandatory = True,
        ),
        "tag": attr.string(
            doc = "Image tag baked into the layout annotation.",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:oci_deploy.py",
        ),
    },
    doc = "Launcher template expansion for oci_deploy (wrapped as py_binary).",
)

def oci_deploy(name, image_tar, tag = "latest", registry = OCI_DEFAULT_REGISTRY, repository = "", profile = "release"):
    """Publishes one image tar as a local-first OCI deployment.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program` (`py_binary`
    on the managed Python 3.12 toolchain wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the default builds a local
    OCI image-layout directory (`<repo>-oci-layout/` with `oci-layout`,
    `index.json`, and `blobs/sha256/`) and verifies bytes, publishing
    nothing. Live `docker push` plus `cosign sign <digest>` runs only with
    `OCI_PUBLISH_LIVE=1`, `OCI_REGISTRY_USER`, `OCI_REGISTRY_TOKEN`, and
    `OCI_PUBLISH_APPROVED=1` after explicit owner approval, with
    SBOM/provenance plus signing verification first. Registry credentials
    come from env only, never from BUILD."""
    effective_repo = repository if repository != "" else name
    tag_error = oci_tag_error(tag)
    if tag_error != "":
        fail(tag_error + " (in " + native.package_name() + ":" + name + ")")
    registry_error = oci_registry_error(registry)
    if registry_error != "":
        fail(registry_error + " (in " + native.package_name() + ":" + name + ")")
    repo_error = oci_repository_error(effective_repo)
    if repo_error != "":
        fail(repo_error + " (in " + native.package_name() + ":" + name + ")")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _oci_launcher(
        name = launcher_target,
        image_tar = image_tar,
        registry = registry,
        repository = effective_repo,
        tag = tag,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = [image_tar],
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
