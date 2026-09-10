"""Analysis subject exposing an aggregate workspace policy (M03 WP1).

Renders every family section as `family.<id>.<capability>` fields with
comma-joined tool IDs in policy order, so `starlark_test` analysis mode
pins the frozen aggregate shape. Empty selections render as empty fields,
which is the explicit-disable encoding.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load(":policy.bzl", "CAPABILITIES", "QualityPolicyInfo")

def _policy_subject_impl(ctx):
    fields = {}
    for family_id in sorted(ctx.attr.policy[QualityPolicyInfo].families.keys()):
        section = ctx.attr.policy[QualityPolicyInfo].families[family_id]
        for capability in CAPABILITIES:
            fields["family." + family_id + "." + capability] = ",".join(
                getattr(section, capability),
            )
    return [
        DefaultInfo(files = depset([])),
        DxSubjectInfo(fields = fields),
    ]

policy_subject = rule(
    implementation = _policy_subject_impl,
    attrs = {
        "policy": attr.label(
            providers = [QualityPolicyInfo],
        ),
    },
)
