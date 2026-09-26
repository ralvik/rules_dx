
CAPABILITIES = ["lint", "typecheck", "format", "audit"]

FamilyPolicyInfo = provider(
    fields = {
        "audit": "List[str]: selected audit tool IDs, in policy order; [] disables.",
        "family_id": "Str: policy-family ID owning this selection.",
        "format": "List[str]: selected formatter IDs, in policy order; [] disables.",
        "lint": "List[str]: selected lint tool IDs, in policy order; [] disables.",
        "typecheck": "List[str]: selected typecheck tool IDs, in policy order; [] disables.",
    },
)

QualityPolicyInfo = provider(
    fields = {
        "families": "Dict[str, FamilyPolicyInfo]: family ID to its section.",
    },
)

def _check_tool_ids(family_id, capability, tool_ids):
    seen = {}
    for tool_id in tool_ids:
        if type(tool_id) != "string" or tool_id == "":
            fail("quality_family (" + family_id + "): capability '" +
                 capability + "' holds a non-string or empty tool ID")
        if tool_id in seen:
            fail("quality_family (" + family_id + "): capability '" +
                 capability + "' selects tool '" + tool_id + "' twice")
        seen[tool_id] = True

def _quality_family_impl(ctx):
    family_id = ctx.attr.family_id
    if type(family_id) != "string" or family_id == "":
        fail("quality_family: family_id must be a non-empty string")
    for capability in CAPABILITIES:
        _check_tool_ids(family_id, capability, getattr(ctx.attr, capability))
    return [FamilyPolicyInfo(
        family_id = family_id,
        lint = ctx.attr.lint,
        typecheck = ctx.attr.typecheck,
        format = ctx.attr.format,
        audit = ctx.attr.audit,
    )]

quality_family = rule(
    implementation = _quality_family_impl,
    attrs = {
        "audit": attr.string_list(
            default = [],
        ),
        "family_id": attr.string(
        ),
        "format": attr.string_list(
            default = [],
        ),
        "lint": attr.string_list(
            default = [],
        ),
        "typecheck": attr.string_list(
            default = [],
        ),
    },
)

def _workspace_policy_impl(ctx):
    families = {}
    for section in ctx.attr.families:
        info = section[FamilyPolicyInfo]
        if info.family_id in families:
            fail("workspace_policy: family '" + info.family_id +
                 "' is provided twice")
        families[info.family_id] = info
    return [QualityPolicyInfo(families = families)]

workspace_policy = rule(
    implementation = _workspace_policy_impl,
    attrs = {
        "families": attr.label_list(
            providers = [FamilyPolicyInfo],
            default = [],
        ),
    },
)
