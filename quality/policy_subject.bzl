"""Analysis subject exposing an aggregate workspace policy (WP1).

Contract: `docs/quality/tool-integrations.md`, `docs/decisions/0011-configuration-composition.md`.
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
            doc = "Aggregate workspace policy whose families render as subject fields.",
        ),
    },
    doc = "Analysis subject exposing an aggregate workspace policy (WP1).",
)
