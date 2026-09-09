"""Workspace quality-policy providers (M03 freeze for O17).

Typed per-family sections plus a thin canonical aggregate, per ADR 0011.
A `quality_family` selects stable tool IDs per capability for one policy
family; a `workspace_policy` aggregates family sections into the canonical
target selected through `@rules_dx//config:workspace`. Empty capability
lists explicitly disable that capability for the family; an omitted family
means curated defaults apply (owned by the tool baseline, M04+).

Fail-fast validation at construction: non-string, empty, or duplicate tool
IDs fail the family; empty or duplicate family IDs fail the aggregate.
Unknown tool IDs fail later at adapter matching during analysis (see
`applicability.bzl`), which the tool-integrations contract accepts as
"during configuration or analysis".

Contract: `docs/quality/tool-integrations.md`. The family taxonomy (which
families exist) and curated defaults are not frozen here.
"""

CAPABILITIES = ["lint", "typecheck", "format", "audit"]

FamilyPolicyInfo = provider(
    doc = "Tool-ID selection per capability for one quality policy family.",
    fields = {
        "family_id": "Str: policy-family ID owning this selection.",
        "lint": "List[str]: selected lint tool IDs, in policy order; [] disables.",
        "typecheck": "List[str]: selected typecheck tool IDs, in policy order; [] disables.",
        "format": "List[str]: selected formatter IDs, in policy order; [] disables.",
        "audit": "List[str]: selected audit tool IDs, in policy order; [] disables.",
    },
)

QualityPolicyInfo = provider(
    doc = "Canonical aggregate workspace policy: family ID to section.",
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
        "family_id": attr.string(
            doc = "Policy-family ID owning this selection.",
        ),
        "lint": attr.string_list(
            default = [],
            doc = "Selected lint tool IDs; [] explicitly disables lint.",
        ),
        "typecheck": attr.string_list(
            default = [],
            doc = "Selected typecheck tool IDs; [] explicitly disables typecheck.",
        ),
        "format": attr.string_list(
            default = [],
            doc = "Selected formatter IDs; [] explicitly disables format.",
        ),
        "audit": attr.string_list(
            default = [],
            doc = "Selected audit tool IDs; [] explicitly disables audit.",
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
            doc = "Family sections; each family_id must appear at most once.",
        ),
    },
)
