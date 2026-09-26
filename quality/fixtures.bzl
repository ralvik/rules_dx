"""Source fixtures for aspects (synthetic, real).

Contract: `docs/quality/quality-sources.md`.
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
            allow_files = [".py"],
            default = [],
            doc = "Directly owned Python sources for this fixture target.",
        ),
        "rust_srcs": attr.label_list(
            allow_files = [".rs"],
            default = [],
            doc = "Directly owned Rust sources for this fixture target.",
        ),
    },
    doc = "Minimal QualitySourcesInfo fixture for aspect evidence.",
)

def _real_source_target_impl(ctx):
    # File-family families without dedicated wrappers stay fixture- and
    # contract: quality/wrapper_owners.bzl): this rule covers the
    # aspect-wired core, runner matrices cover the rest, never a second
    # cc/rules/defs.bzl), not fixture-owned here.
    direct_sources = {}
    if len(ctx.files.javascript_srcs) > 0:
        direct_sources["javascript"] = depset(ctx.files.javascript_srcs)
    if len(ctx.files.jsx_srcs) > 0:
        direct_sources["jsx"] = depset(ctx.files.jsx_srcs)
    if len(ctx.files.typescript_srcs) > 0:
        direct_sources["typescript"] = depset(ctx.files.typescript_srcs)
    if len(ctx.files.tsx_srcs) > 0:
        direct_sources["tsx"] = depset(ctx.files.tsx_srcs)
    if len(ctx.files.json_srcs) > 0:
        direct_sources["json"] = depset(ctx.files.json_srcs)
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
    if len(ctx.files.java_srcs) > 0:
        direct_sources["java"] = depset(ctx.files.java_srcs)
    if len(ctx.files.kotlin_srcs) > 0:
        direct_sources["kotlin"] = depset(ctx.files.kotlin_srcs)
    check_direct_sources(direct_sources, str(ctx.label))
    all_files = list(ctx.files.javascript_srcs) + list(ctx.files.jsx_srcs) + list(ctx.files.typescript_srcs) + list(ctx.files.tsx_srcs) + list(ctx.files.json_srcs) + list(ctx.files.python_srcs) + list(ctx.files.python_stub_srcs) + list(ctx.files.rust_srcs) + list(ctx.files.starlark_srcs) + list(ctx.files.toml_srcs) + list(ctx.files.markdown_srcs) + list(ctx.files.java_srcs) + list(ctx.files.kotlin_srcs)
    return [
        DefaultInfo(files = depset(all_files)),
        QualitySourcesInfo(direct_sources = direct_sources),
    ]

real_source_target = rule(
    implementation = _real_source_target_impl,
    attrs = {
        "java_srcs": attr.label_list(
            allow_files = [".java"],
            default = [],
            doc = "Directly owned Java sources for this fixture target.",
        ),
        "javascript_srcs": attr.label_list(
            allow_files = [".js", ".mjs", ".cjs"],
            default = [],
            doc = "Directly owned JavaScript sources for this fixture target.",
        ),
        "kotlin_srcs": attr.label_list(
            allow_files = [".kt", ".kts"],
            default = [],
            doc = "Directly owned Kotlin sources for this fixture target.",
        ),
        "json_srcs": attr.label_list(
            allow_files = [".json"],
            default = [],
            doc = "Directly owned JSON sources for this fixture target.",
        ),
        "jsx_srcs": attr.label_list(
            allow_files = [".jsx"],
            default = [],
            doc = "Directly owned JSX sources for this fixture target.",
        ),
        "markdown_siblings": attr.label_list(
            allow_files = True,
            default = [],
            doc = "Unclassified Markdown link-resolution siblings (for example a LICENSE file): mirrored into the check for target resolution, never linted, never in findings or snapshots. Unbounded by design: siblings carry no linted extension. See: issue #928.",
        ),
        "markdown_srcs": attr.label_list(
            allow_files = [".md"],
            default = [],
            doc = "Directly owned Markdown sources for this fixture target.",
        ),
        "python_srcs": attr.label_list(
            allow_files = [".py"],
            default = [],
            doc = "Directly owned Python sources for this fixture target.",
        ),
        "python_stub_srcs": attr.label_list(
            allow_files = [".pyi"],
            default = [],
            doc = "Directly owned Python stub sources for this fixture target.",
        ),
        "rust_srcs": attr.label_list(
            allow_files = [".rs"],
            default = [],
            doc = "Directly owned Rust sources for this fixture target.",
        ),
        "starlark_srcs": attr.label_list(
            allow_files = [".bzl", ".bazel"],
            default = [],
            doc = "Directly owned Starlark sources for this fixture target (BUILD.bazel plus .bzl).",
        ),
        "toml_srcs": attr.label_list(
            allow_files = [".toml"],
            default = [],
            doc = "Directly owned TOML sources for this fixture target.",
        ),
        "tsx_srcs": attr.label_list(
            allow_files = [".tsx"],
            default = [],
            doc = "Directly owned TSX sources for this fixture target.",
        ),
        "typescript_srcs": attr.label_list(
            allow_files = [".ts", ".mts", ".cts"],
            default = [],
            doc = "Directly owned TypeScript sources for this fixture target (declaration files are inert and rejected by the wrapper).",
        ),
    },
    doc = "Minimal QualitySourcesInfo fixture with native-config hints for real aspect evidence.",
)
