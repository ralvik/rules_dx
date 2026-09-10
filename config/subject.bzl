"""Analysis subject exposing the canonical quality settings (M03 WP1).

Reads the `@rules_dx//config` build settings and republishes them as
`DxSubjectInfo` fields so `starlark_test` analysis mode can pin the frozen
defaults. The `workspace` value is reported as the canonical label string;
it names the consumer's aggregate policy target and is not resolved here.
"""

load("@bazel_skylib//rules:common_settings.bzl", "BuildSettingInfo")
load("//libs/starlark:defs.bzl", "DxSubjectInfo")

def _settings_subject_impl(ctx):
    return [
        DefaultInfo(files = depset([])),
        DxSubjectInfo(fields = {
            "fail_on": ctx.attr._fail_on[BuildSettingInfo].value,
            "validate": str(ctx.attr._validate[BuildSettingInfo].value),
            "workspace": str(ctx.attr._workspace.label),
        }),
    ]

settings_subject = rule(
    implementation = _settings_subject_impl,
    attrs = {
        "_fail_on": attr.label(default = "//config:fail_on"),
        "_validate": attr.label(default = "//config:validate"),
        "_workspace": attr.label(default = "//config:workspace"),
    },
)
