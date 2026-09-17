"""Draft-only GitHub Release publisher for `dx deploy` (issue #182).

`github_release` is the second deploy macro: it wraps
`gh release create --draft --verify-tag`, returning `DxDeployInfo` so the
release target runs under `bazel run` and `dx deploy` like any other
deployment. The `gh` CLI is a host tool resolved at run time (like `tar`
for `archive_release`); no registry, no new module dependencies.

Safety (issue #5): the macro is draft-only by construction. `draft`
must stay `True` (analysis fails otherwise), every invocation passes
`--verify-tag` so the program never creates or pushes tags itself, and
the default tag is the `v0.0.0-dryrun` placeholder so an accidental run
cannot touch a real release. CI proves the program reports its command
via `GH_RELEASE_DRY_RUN=1` without network access; nothing runs the
publisher on push/PR. A real draft needs the tag pushed beforehand and
explicit owner approval, then publishing happens by editing the draft
on GitHub.

Contract: `docs/deploy/authoring.md`. Deploy targets live next to the
app they release (for example `//dx/cli:github_draft`).
"""

load(":defs.bzl", "dx_deployment")

# Release tags embed directly in the generated launcher, so the charset
# is restricted to what is safe inside double quotes.
_VALID_TAG_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"

def github_tag_error(tag):
    """Validates one release tag value.

    Args:
      tag: candidate tag string.

    Returns:
      "" when valid, else the failure reason naming the bad value.
    """
    if type(tag) != "string" or tag == "":
        return ("github_release: invalid tag '" + str(tag) +
                "': want a non-empty tag (for example 'v0.0.0-dryrun')")
    for c in tag.elems():
        if c not in _VALID_TAG_CHARS:
            return ("github_release: invalid tag '" + tag +
                    "': want only [A-Za-z0-9._-] so the tag embeds " +
                    "safely in the deploy launcher")
    return ""

def github_draft_error(draft):
    """Validates the draft gate.

    Args:
      draft: candidate draft flag; only True is accepted.

    Returns:
      "" when valid, else the failure reason.
    """
    if draft != True:
        return ("github_release: draft=False requires explicit owner " +
                "approval per issue #5; keep the draft gate and publish " +
                "the release on GitHub after approval")
    return ""

def _github_rlocation(ctx, f):
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

def _github_program_impl(ctx):
    """Writes the executable deploy launcher for one draft release.

    Each artifact resolves to a single file: executables (for example
    `rust_binary`, `sh_binary`) resolve to `files_to_run.executable`,
    plain files (for example `archive_release` tarballs) must be the
    sole member of `DefaultInfo.files`. Like `archive_release`,
    `sh_binary(args=...)` cannot carry paths through `dx_deployment`
    (which symlinks only `files_to_run.executable`), so the launcher
    resolves the deploy script and every asset from its own runfiles
    forest and execs `github_deploy.sh` with the tag plus the asset
    paths. Extra user args after `--` are rejected: a release takes
    exactly the artifacts pinned at analysis time.

    Args:
      ctx: rule context with `artifacts`, `tag`, `deploy_sh`.

    Returns:
      `DefaultInfo` with the executable launcher and its runfiles.
    """
    asset_files = []
    asset_rlocs = []
    runfiles = ctx.runfiles()
    for target in ctx.attr.artifacts:
        info = target[DefaultInfo]
        f = info.files_to_run.executable
        if f == None:
            files = info.files.to_list()
            if len(files) != 1:
                fail("github_release " + str(ctx.label) + ": artifact " +
                     str(target.label) + " provides " +
                     str(len(files)) + " files, want exactly one " +
                     "(executables resolve to their binary)")
            f = files[0]
        asset_files.append(f)
        asset_rlocs.append(_github_rlocation(ctx, f))
        runfiles = runfiles.merge(info.default_runfiles)
    deploy_file = ctx.file.deploy_sh
    deploy_rloc = _github_rlocation(ctx, deploy_file)
    runfiles = runfiles.merge(
        ctx.runfiles(files = asset_files + [deploy_file]),
    )

    asset_lines = "".join(
        ["  \"$(rloc \"%s\")\"\n" % rloc for rloc in asset_rlocs],
    )
    launcher = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.write(
        output = launcher,
        content = """#!/usr/bin/env bash
# Deploy launcher for `github_release` (issue #182). Generated. Do not edit.
# Resolves the deploy script and every pinned asset from this launcher's
# runfiles forest, then execs the deploy script with the tag plus the
# asset paths. `dx_deployment` symlinks only the executable, so
# `sh_binary` `args` cannot survive; runfiles lookup keeps the paths
# intact. Works under `bazel run`, `dx deploy`, and direct `bazel-bin`
# execution via `RUNFILES_DIR` / `$0.runfiles` / manifest.
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
  echo "github: cannot locate runfiles (tried RUNFILES_DIR, $0.runfiles)" >&2
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
if [[ "$#" -gt 0 ]]; then
  echo "github: this deploy target takes no extra args; the release is exactly the artifacts pinned at analysis time" >&2
  exit 1
fi
DEPLOY="$(rloc "%s")"
TAG="%s"
ASSETS=(
%s)
exec "${DEPLOY}" "${TAG}" "${ASSETS[@]}"
""" % (deploy_rloc, ctx.attr.tag, asset_lines),
        is_executable = True,
    )
    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

_github_program = rule(
    implementation = _github_program_impl,
    executable = True,
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
    doc = "Executable deploy launcher for github_release (runfiles-resolved).",
)

def github_release(name, artifacts, tag = "v0.0.0-dryrun", draft = True, profile = "release"):
    """Publishes pinned files as a draft-only GitHub Release.

    Creates `<name>_program` (runfiles-resolved deploy launcher) and
    `<name>` (the `dx_deployment` returning `DxDeployInfo` with no app
    and `profile`). Run with `bazel run :<name>` or `dx deploy :<name>`;
    the program execs `gh release create <tag> <assets...> --draft
    --verify-tag`. With `GH_RELEASE_DRY_RUN=1` it prints the command and
    publishes nothing (this is what CI exercises).

    Args:
      name: instance name; also the deploy target name.
      artifacts: labels of the release asset files.
      tag: release tag; defaults to the dry-run placeholder.
      draft: must stay True (issue #5); False fails analysis.
      profile: default profile (`debug`, `dev`, or `release`).
    """
    tag_error = github_tag_error(tag)
    if tag_error != "":
        fail(tag_error + " (in " + native.package_name() + ":" + name + ")")
    draft_error = github_draft_error(draft)
    if draft_error != "":
        fail(draft_error + " (in " + native.package_name() + ":" + name + ")")
    if len(artifacts) == 0:
        fail("github_release " + native.package_name() + ":" + name +
             ": need at least one artifact")

    program_target = name + "_program"
    _github_program(
        name = program_target,
        artifacts = artifacts,
        tag = tag,
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
