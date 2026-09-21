"""Draft-only GitHub Release publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load(":defs.bzl", "dx_deployment")
load(":launcher.bzl", "RUNFILES_BASH_INIT", "rlocation_path")

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
    tags embed safely in the deploy launcher."""
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
    """Writes the `sh_binary` launcher script for one draft release.

    Each artifact resolves to a single file: executables (for example
    `rust_binary`, `sh_binary`) resolve to `files_to_run.executable`,
    plain files (for example `archive_deploy` tarballs) must be the
    sole member of `DefaultInfo.files`. The script sources the standard
    `runfiles.bash` initialization (v3) and resolves the deploy script,
    tag, and every pinned asset via `rlocation`, then execs
    `github_deploy.sh`. Rlocation strings and the tag embed with
    `shell.quote` (single-quote), never manual double-quote
    interpolation. The wrapping `sh_binary` (see `github_deploy`)
    carries the pinned inputs in `data` plus the runfiles library.
    Extra user args after `--` are rejected: a release takes exactly
    the artifacts pinned at analysis time."""
    asset_files = []
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
        asset_files.append(f)
        asset_rlocs.append(rlocation_path(ctx, f))
    deploy_file = ctx.file.deploy_sh
    deploy_rloc = rlocation_path(ctx, deploy_file)

    asset_lines = "".join(
        ["  \"$(rlocation " + shell.quote(rloc) + ")\"\n" for rloc in asset_rlocs],
    )
    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `github_deploy`. Generated. Do not edit.
# Resolves the deploy script and every pinned asset via the standard
# `runfiles.bash` `rlocation`, then execs the deploy script with the tag
# plus the asset paths. Wrapped as `sh_binary` (see `github_deploy`).
set -euo pipefail
""" + RUNFILES_BASH_INIT + """if [[ "$#" -gt 0 ]]; then
  echo "github: this deploy target takes no extra args; the release is exactly the artifacts pinned at analysis time" >&2
  exit 1
fi
DEPLOY="$(rlocation """ + shell.quote(deploy_rloc) + """)"
TAG=""" + shell.quote(ctx.attr.tag) + """
ASSETS=(
""" + asset_lines + """)
exec "${DEPLOY}" "${TAG}" "${ASSETS[@]}"
""",
        is_executable = True,
    )
    return [DefaultInfo(files = depset([launcher]))]

_github_launcher = rule(
    implementation = _github_launcher_impl,
    attrs = {
        "artifacts": attr.label_list(
            doc = "Release asset files (executables resolve to their binary).",
            mandatory = True,
        ),
        "tag": attr.string(
            doc = "Release tag; must already exist in the remote (--verify-tag).",
            mandatory = True,
        ),
        "deploy_sh": attr.label(
            allow_single_file = True,
            default = "//deploy/rules:github_deploy.sh",
        ),
    },
    doc = "Launcher script for github_deploy (wrapped as sh_binary).",
)

def github_deploy(name, artifacts, tag = "v0.0.0-dryrun", draft = True, profile = "release"):
    """Publishes pinned files as a draft-only GitHub Release.

    Creates `<name>_launcher` (generated launcher script resolving
    inputs via `runfiles.bash` `rlocation` with `shell.quote`),
    `<name>_program` (`sh_binary` wrapping the launcher with pinned
    `data` plus the runfiles library), and `<name>` (the `dx_deployment`
    returning `DxDeployInfo` with no app and `profile`). Run with
    `bazel run :<name>` or `dx deploy :<name>`; the program execs
    `gh release create <tag> <assets...> --draft --verify-tag`. With
    `GH_RELEASE_DRY_RUN=1` it prints the command and publishes nothing
    (this is what CI exercises)."""
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
        tag = tag,
    )

    # `sh_binary` wrapper: `srcs` is the generated launcher,
    # `data` pins the runfiles the launcher resolves via `rlocation`
    # (location expansion), `deps` carries the standard runfiles library.
    sh_binary(
        name = program_target,
        srcs = [":" + launcher_target],
        data = artifacts + ["//deploy/rules:github_deploy.sh"],
        deps = ["@rules_shell//shell/runfiles"],
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
