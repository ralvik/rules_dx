"""Deploy boundary for `dx deploy` (issue #178).

`DxDeployInfo` is the narrow public boundary deploy macros target,
parallel to `QualitySourcesInfo` for quality workflows. It carries only
the deploy entrypoint's identity (`app`) and its default profile
(`profile`); it does not carry kind, environment, or veto metadata.
`dx deploy` dispatches on this provider or on raw executability, so
user-defined deployers participate without depending on private rules.

Contract: `docs/deploy/authoring.md`. The provider concept and the
`deploy/rules:defs.bzl` load label are frozen here. Profile vocabulary
(`debug`, `dev`, `release`) follows ADR 0021; the flag-over-attribute
precedence and `DX_PROFILE` forwarding belong to issue #179, not here.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

DxDeployInfo = provider(
    doc = "Deploy entrypoint identity and default profile for `dx deploy` dispatch.",
    fields = {
        "app": "Label or None: the deployed app target when distinct from the deploy target.",
        "profile": "String or None: default profile ('debug', 'dev', 'release'); None means the command default applies.",
    },
)

# Profile vocabulary from ADR 0021. `dx_dev` equals the Bazel default
# `fastbuild` for the inner loop; `dx_release` (`opt`) is the deploy
# default; `dx_debug` (`dbg`) is diagnostics. The `dx_` config prefix is
# a Bazel-config name only; provider and CLI spellings stay bare.
VALID_DEPLOY_PROFILES = ["debug", "dev", "release"]

def deploy_profile_error(profile):
    """Validates one deploy profile value.

    Args:
      profile: candidate profile string, or None meaning the command
        default applies (custom rules may omit the default).

    Returns:
      "" when valid, else the failure reason naming the bad value.
    """
    if profile == None:
        return ""
    if type(profile) != "string" or profile not in VALID_DEPLOY_PROFILES:
        return ("dx_deployment: invalid profile '" + str(profile) +
                "': want one of debug, dev, release")
    return ""

def _dx_deployment_impl(ctx):
    error = deploy_profile_error(ctx.attr.profile)
    if error != "":
        fail(error + " (in " + str(ctx.label) + ")")
    deploy_default = ctx.attr.deploy[DefaultInfo]
    exe = deploy_default.files_to_run.executable
    if exe == None:
        fail("dx_deployment " + str(ctx.label) + ": deploy target " +
             str(ctx.attr.deploy.label) + " has no executable")
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    runfiles = ctx.runfiles(files = [link]).merge(deploy_default.default_runfiles)
    app_label = None
    app_text = ""
    if ctx.attr.app:
        app_label = ctx.attr.app.label
        app_text = display_label(app_label)
    return [
        DefaultInfo(
            executable = link,
            files = depset([link]),
            runfiles = runfiles,
        ),
        DxDeployInfo(app = app_label, profile = ctx.attr.profile),
        DxSubjectInfo(fields = {
            "app": app_text,
            "profile": ctx.attr.profile,
        }),
    ]

dx_deployment = rule(
    implementation = _dx_deployment_impl,
    executable = True,
    attrs = {
        "app": attr.label(
            doc = "Deployed app target when distinct from the deploy program; None deploys the program itself.",
            mandatory = False,
        ),
        "deploy": attr.label(
            cfg = "target",
            doc = "Executable deploy program this deployment runs.",
            executable = True,
            mandatory = True,
        ),
        "profile": attr.string(
            default = "release",
            doc = "Default profile for this deployment (debug, dev, or release). An explicit CLI flag always wins.",
        ),
    },
    doc = "Wraps one executable deploy program with DxDeployInfo (issue #178).",
)
