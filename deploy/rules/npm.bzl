"""Hermetic npm pack feed publisher for `dx deploy`.

Contract: `docs/deploy/authoring.md`.
"""

load("@bazel_skylib//lib:shell.bzl", "shell")
load("@rules_python//python:defs.bzl", "py_binary")
load(":defs.bzl", "dx_deployment")

_VALID_TAG_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
_VALID_PACKAGE_CHARS = _VALID_TAG_CHARS + "@/"

def npm_tag_error(tag):
    """Validates one npm dist-tag value.

    Args:
      tag: Candidate npm dist-tag (for example `latest`).

    Returns:
      Empty string when valid, else an actionable error message.
    """
    if type(tag) != "string" or tag == "":
        return ("npm_deploy: invalid tag '" + str(tag) +
                "': want a non-empty tag (for example 'latest')")
    for c in tag.elems():
        if c not in _VALID_TAG_CHARS:
            return ("npm_deploy: invalid tag '" + tag +
                    "': want only [A-Za-z0-9._-] so the tag embeds " +
                    "safely in the deploy program")
    return ""

def npm_package_error(package):
    """Validates one npm package name value.

    Args:
      package: Candidate npm package name.

    Returns:
      Empty string when valid, else an actionable error message.
    """
    if type(package) != "string" or package == "":
        return ("npm_deploy: invalid package '" + str(package) +
                "': want a non-empty package name (for example 'npm-demo')")
    for c in package.elems():
        if c not in _VALID_PACKAGE_CHARS:
            return ("npm_deploy: invalid package '" + package +
                    "': want only [A-Za-z0-9@/_.-] so the package name " +
                    "embeds safely in the deploy program")
    return ""

def npm_registry_error(registry):
    """Validates one npm registry URL value.

    Args:
      registry: Candidate https registry URL.

    Returns:
      Empty string when valid, else an actionable error message.
    """
    if type(registry) != "string" or not registry.startswith("https://"):
        return ("npm_deploy: invalid registry '" + str(registry) +
                "': want an https:// URL " +
                "(for example 'https://registry.npmjs.org')")
    for c in registry.elems():
        if c in ["\"", "\\", "'", " ", "\n", "`", "$"]:
            return ("npm_deploy: invalid registry '" + registry +
                    "': want an https:// URL " +
                    "(for example 'https://registry.npmjs.org')")
    return ""

def npm_filenames(name):
    """Returns the deterministic (tarball, feed) output names."""
    return (name + ".tgz", name + ".feed.json")

def npm_deploy(name, package, tag = "latest", srcs = [], registry = "https://registry.npmjs.org", profile = "release"):
    """Publishes packed files to a local npm folder feed.

    Creates `<name>_pack` (deterministic tgz + feed JSON via the
    hermetic `//deploy/rules:npm_packer` tool), `<name>_program`
    (`py_binary` verifying bytes and assembling the folder feed, live
    `npm publish --access public --provenance` only with
    `NPM_PUBLISH_LIVE=1` plus `NPM_TOKEN`), and `<name>` (the
    `dx_deployment` returning `DxDeployInfo`). Run with
    `bazel run :<name>` or `dx deploy :<name>`.

    Args:
      name: Deploy target base name.
      package: npm package name for the packed feed.
      tag: npm dist-tag for the publish path.
      srcs: Source labels packed into the tarball.
      registry: https registry URL for the publish path.
      profile: Deploy profile (debug, dev, or release).
    """
    pkg_error = npm_package_error(package)
    if pkg_error != "":
        fail(pkg_error + " (in " + native.package_name() + ":" + name + ")")
    tag_error = npm_tag_error(tag)
    if tag_error != "":
        fail(tag_error + " (in " + native.package_name() + ":" + name + ")")
    registry_error = npm_registry_error(registry)
    if registry_error != "":
        fail(registry_error + " (in " + native.package_name() + ":" + name + ")")
    if len(srcs) == 0:
        fail("npm_deploy " + native.package_name() + ":" + name +
             ": need at least one src")

    (tarball, feed) = npm_filenames(name)
    pack_target = name + "_pack"
    program_target = name + "_program"

    native.genrule(
        name = pack_target,
        srcs = srcs,
        outs = [tarball, feed],
        tools = ["//deploy/rules:npm_packer"],
        cmd = "$(location //deploy/rules:npm_packer) " + shell.quote(package) + " " + shell.quote(tag) + " " + shell.quote(registry) + " $(OUTS) $(SRCS)",
    )

    py_binary(
        name = program_target,
        srcs = ["//deploy/rules:npm_deploy.py"],
        main = "//deploy/rules:npm_deploy.py",
        data = [":" + pack_target],
    )

    dx_deployment(
        name = name,
        deploy = ":" + program_target,
        profile = profile,
    )
