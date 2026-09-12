"""Source fixtures for aspects (M03 synthetic, M04 real).

`quality_source_target` is a minimal custom rule proving custom-rule
integration solely through `QualitySourcesInfo`: no wrapper, no language
toolchain, no generic `srcs` fallback. Aspects read only this provider.

`real_source_target` is the M04 counterpart over the initial-adapter
classes (rust, starlark, toml, markdown) with `aspect_hints` for typed
native configs. Real aspects read only `QualitySourcesInfo` plus hints.
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

def _real_source_target_impl(ctx):
    direct_sources = {}
    if len(ctx.files.python_srcs) > 0:
        direct_sources["python"] = depset(ctx.files.python_srcs)
    if len(ctx.files.python_stub_srcs) > 0:
        direct_sources["python_stub"] = depset(ctx.files.python_stub_srcs)
    if len(ctx.files.rust_srcs) > 0:
        direct_sources["rust"] = depset(ctx.files.rust_srcs)
    if len(ctx.files.starlark_srcs) > 0:
        direct_sources["starlark"] = depset(ctx.files.starlark_srcs)
    if len(ctx.files.toml_srcs) > 0:
        direct_sources["toml"] = depset(ctx.files.toml_srcs)
    if len(ctx.files.markdown_srcs) > 0:
        direct_sources["markdown"] = depset(ctx.files.markdown_srcs)
    check_direct_sources(direct_sources, str(ctx.label))
    all_files = list(ctx.files.python_srcs) + list(ctx.files.python_stub_srcs) + list(ctx.files.rust_srcs) + list(ctx.files.starlark_srcs) + list(ctx.files.toml_srcs) + list(ctx.files.markdown_srcs)
    return [
        DefaultInfo(files = depset(all_files)),
        QualitySourcesInfo(direct_sources = direct_sources),
    ]

real_source_target = rule(
    implementation = _real_source_target_impl,
    attrs = {
        "markdown_siblings": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Unclassified Markdown link-resolution siblings (for example a LICENSE file): mirrored into the check for target resolution, never linted, never in findings or snapshots.",
        ),
        "markdown_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Markdown sources for this fixture target.",
        ),
        "python_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Python sources for this fixture target.",
        ),
        "python_stub_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Python stub sources for this fixture target.",
        ),
        "rust_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Rust sources for this fixture target.",
        ),
        "starlark_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned Starlark sources for this fixture target.",
        ),
        "toml_srcs": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Directly owned TOML sources for this fixture target.",
        ),
    },
    doc = "Minimal QualitySourcesInfo fixture with native-config hints for real aspect evidence.",
)
