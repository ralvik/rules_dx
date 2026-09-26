"""Focused Rust environment plan (WP3).

"""

load("@rules_rust//rust:defs.bzl", _rust_common = "rust_common")
load("//env:focused.bzl", "focused_direct_sources", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

RustEnvPlanInfo = provider(
    doc = "Provider-derived focused Rust target environment plan.",
    fields = {
        "crate_name": "Crate name from authoritative CrateInfo.",
        "crate_type": "Crate type from authoritative CrateInfo.",
        "direct_dep_count": "Number of direct crate deps (provider length).",
        "direct_sources": "Sorted basenames of direct Rust sources.",
        "edition": "Crate edition from authoritative CrateInfo.",
        "root": "Crate root basename from authoritative CrateInfo.",
        "target": "Display label of the planned wrapper target.",
    },
)

def _crate_of(target):
    if _rust_common.crate_info in target:
        return target[_rust_common.crate_info], False
    if _rust_common.test_crate_info in target:
        return target[_rust_common.test_crate_info].crate, True
    fail("rust_env_plan: target has neither CrateInfo nor TestCrateInfo: " +
         display_label(target.label))

def _rust_env_plan_impl(ctx):
    target = ctx.attr.target
    crate, via_test_crate = _crate_of(target)
    sources = focused_direct_sources(target)
    plan = {
        "crate_name": crate.name,
        "crate_type": crate.type,
        "direct_dep_count": str(len(crate.deps.to_list())),
        "direct_sources": ",".join(sources),
        "edition": crate.edition,
        "root": crate.root.basename,
        "target": display_label(ctx.attr.target.label),
        "via_test_crate": str(via_test_crate),
    }
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        RustEnvPlanInfo(
            crate_name = plan["crate_name"],
            crate_type = plan["crate_type"],
            direct_dep_count = plan["direct_dep_count"],
            direct_sources = sources,
            edition = plan["edition"],
            root = plan["root"],
            target = plan["target"],
        ),
        DxSubjectInfo(fields = plan),
    ]

rust_env_plan = rule(
    implementation = _rust_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One rust_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused Rust environment plan for one wrapper target (WP3).",
)
