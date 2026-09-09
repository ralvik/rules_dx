"""Synthetic source fixtures for WP2c aspects (M03).

`quality_source_target` is a minimal custom rule proving custom-rule
integration solely through `QualitySourcesInfo`: no wrapper, no language
toolchain, no generic `srcs` fallback. Aspects read only this provider.
"""

load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

def _quality_source_target_impl(ctx):
    direct_sources = {}
    if len(ctx.files.python_srcs) > 0:
        direct_sources["python"] = depset(ctx.files.python_srcs)
    if len(ctx.files.rust_srcs) > 0:
        direct_sources["rust"] = depset(ctx.files.rust_srcs)
    check_direct_sources(direct_sources, str(ctx.label))
    all_files = list(ctx.files.python_srcs) + list(ctx.files.rust_srcs)
    return [
        DefaultInfo(files = depset(all_files)),
        QualitySourcesInfo(direct_sources = direct_sources),
    ]

quality_source_target = rule(
    implementation = _quality_source_target_impl,
    attrs = {
        "python_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Python sources for this fixture target.",
        ),
        "rust_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Rust sources for this fixture target.",
        ),
    },
    doc = "Minimal QualitySourcesInfo fixture for aspect evidence.",
)
