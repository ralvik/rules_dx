"""Draft-only GitHub Release publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "rlocation_path")

# Versioned tag-charset schema. Consumers query via
# `tag_charset` and `github_tag_error` instead of duplicating the charset,
# so any charset evolution edits this one data constant with schema review,
# never a parallel allowlist.
TAG_SCHEMA_VERSION = 1

# Release tags embed directly in the generated launcher, so the charset
# is restricted to what is safe inside double quotes.
_VALID_TAG_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

def tag_charset():
    """Returns the launcher-safe tag charset via registry query.

 Derived from `_VALID_TAG_CHARS`, never duplicated.
    """
    return _VALID_TAG_CHARS

def tag_schema_error():
    """Validates the versioned tag-charset schema.

    Checks data shape without pinning exact contents: version is v1, the
    charset is non-empty with unique shell-safe characters and never
    admits double-quote, backslash, single-quote, space, or newline so
    tags embed safely in the deploy launcher.
    """
    if TAG_SCHEMA_VERSION != 1:
        return "github tag: unsupported schema v" + str(TAG_SCHEMA_VERSION) + " (want v1)"
    if type(_VALID_TAG_CHARS) != "string" or _VALID_TAG_CHARS == "":
        return "github tag: want a non-empty charset (schema v1)"
    seen = {}
    for c in _VALID_TAG_CHARS.elems():
        if c in seen:
            return "github tag: duplicate charset char '" + c + "'"
        seen[c] = True
        if c in ["\"", "\\", "'", " ", "\n", "`", "$", "!", "#", "&", "|", ";", "<", ">", "(", ")", "[", "]", "{", "}", "*", "?", "~", "^", ":", ",", "/", "+"]:
            return "github tag: unsafe charset char '" + c + "' (must stay launcher-safe)"
    return ""

def github_tag_error(tag):
    """Validates one release tag value."""
    if type(tag) != "string" or tag == "":
        return ("github_deploy: invalid tag '" + str(tag) +
                "': want a non-empty tag (for example 'v0.0.0-dryrun')")
    for c in tag.elems():
        if c not in _VALID_TAG_CHARS:
            return ("github_deploy: invalid tag '" + tag +
                    "': want only [A-Za-z0-9._-] so the tag embeds " +
                    "safely in the deploy launcher")
    return ""

def github_draft_error(draft):
    """Validates the draft gate."""
    if draft != True:
        return ("github_deploy: draft=False requires explicit owner " +
                "approval per issue #5; keep the draft gate and publish " +
                "the release on GitHub after approval")
    return ""

def _github_launcher_impl(ctx):
    """Expands the `py_binary` launcher for one draft release.

    Each artifact resolves to a single file: executables (for example
    `rust_binary`, `py_binary`) resolve to `files_to_run.executable`,
    plain files (for example `archive_deploy` tarballs) must be the
    sole member of `DefaultInfo.files`. The rule computes the runfiles
    rlocations for every pinned asset via `rlocation_path`, then expands
    the shared `github_deploy.py` template with those pins plus the tag
    and deploy name. The wrapping `py_binary` (see `github_deploy`)
    carries the pinned inputs in `data` plus the Python runfiles
    library, so the program works under `bazel run`, `dx deploy` (which
    symlinks the entrypoint and merges its runfiles), and direct
    `bazel-bin` execution. Extra user args after `--` select the output
    directory for the local staging dir (default:
    `$BUILD_WORKSPACE_DIRECTORY`, else the cwd); the default stages
    locally and publishes nothing."""
    asset_rlocs = []
    for target in ctx.attr.artifacts:
        info = target[DefaultInfo]
        f = info.files_to_run.executable
        if f == None:
            files = info.files.to_list()
            if len(files) != 1:
                fail("github_deploy " + str(ctx.label) + ": artifact " +
                     str(target.label) + " provides " +
                     str(len(files)) + " files, want exactly one " +
                     "(executables resolve to their binary)")
            f = files[0]
        asset_rlocs.append(rlocation_path(ctx, f))

    launcher = ctx.actions.declare_file(ctx.label.name + ".py")
    ctx.actions.expand_template(
        template = ctx.file._template,
        output = launcher,
        # buildifier: disable=canonical-repository  # @@KEY@@ are template placeholders, not repo names
        substitutions = {
            "@@ASSET_RLOCS@@": ";".join(asset_rlocs),
            "@@DEPLOY_NAME@@": ctx.attr.deploy_name,
            "@@TAG@@": ctx.attr.tag,
        },
    )
    return [DefaultInfo(files = depset([launcher]))]

_github_launcher = rule(
    implementation = _github_launcher_impl,
    attrs = {
        "artifacts": attr.label_list(
            doc = "Release asset files (executables resolve to their binary).",
            mandatory = True,
        ),
        "deploy_name": attr.string(
            doc = "Deploy target name baked into the local staging directory.",
            mandatory = True,
        ),
        "tag": attr.string(
            doc = "Release tag; must already exist in the remote (--verify-tag).",
            mandatory = True,
        ),
        "_template": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:github_deploy.py",
        ),
    },
    doc = "Launcher template expansion for github_deploy (wrapped as py_binary).",
)

def github_deploy(name, artifacts, tag = "v0.0.0-dryrun", draft = True, profile = "release"):
    """Publishes pinned files as a draft-only GitHub Release.

    Creates `<name>_program_launcher` (expanded Python launcher resolving
    inputs via the Python runfiles library), `<name>_program`
    (`py_binary` on the managed Python 3.12 toolchain wrapping the
    launcher with pinned `data` plus the runfiles library), and `<name>`
    (the `dx_deployment` returning `DxDeployInfo` with no app and
    `profile`). Run with `bazel run :<name>` or `dx deploy :<name>`; the
    default builds a local staging directory (`<name>-release/` holding
    the pinned assets plus `would-run.txt` with the `gh release create
    <tag> <assets...> --draft --verify-tag` manifest) and verifies bytes,
    publishing nothing. `GH_RELEASE_DRY_RUN=1` prints the dry-run header
    and publishes nothing (this is what CI exercises). Live `gh release
    create --draft --verify-tag` runs only with `GH_RELEASE_LIVE=1` and
    `GH_RELEASE_APPROVED=1` after explicit owner approval, never by
    default, and refuses the `v0.0.0-dryrun` placeholder.
    """
    tag_error = github_tag_error(tag)
    if tag_error != "":
        fail(tag_error + " (in " + native.package_name() + ":" + name + ")")
    draft_error = github_draft_error(draft)
    if draft_error != "":
        fail(draft_error + " (in " + native.package_name() + ":" + name + ")")
    if len(artifacts) == 0:
        fail("github_deploy " + native.package_name() + ":" + name +
             ": need at least one artifact")

    program_target = name + "_program"
    launcher_target = program_target + "_launcher"
    _github_launcher(
        name = launcher_target,
        artifacts = artifacts,
        deploy_name = name,
        tag = tag,
    )

    # `py_binary` wrapper: `srcs` is the expanded launcher,
    # `data` pins the runfiles the launcher resolves via `Rlocation`,
    # `deps` carries the Python runfiles library. No shell, no `sh_binary`.
    py_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = artifacts,
        main = launcher_target + ".py",
        deps = ["@rules_python//python/runfiles"],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )

def github_release(name, artifacts, tag = "v0.0.0-dryrun", draft = True, profile = "release"):
    """Compat alias for `github_deploy`.

    Kept for one release cycle, then removed.
    """
    github_deploy(
        name = name,
        artifacts = artifacts,
        tag = tag,
        draft = draft,
        profile = profile,
    )
