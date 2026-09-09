"""Tested-stack manifest generator (M01 WP7, ADR 0014).

Emits a JSON manifest of the pinned tested stack. Version attribute
defaults mirror `.bazelversion` and `MODULE.bazel`; the `stack_contract`
test cross-checks every value against those ground-truth files, so drift
between the manifest and the real pins fails the test.
"""

def _tested_stack_impl(ctx):
    manifest = {
        "schema_version": 1,
        "generator": "tools/testing/tested_stack.bzl",
        "bazel_version": ctx.attr.bazel_version,
        "rules_rust_version": ctx.attr.rules_rust_version,
        "rules_cc_version": ctx.attr.rules_cc_version,
        "rust_version": ctx.attr.rust_version,
        "platforms": sorted(ctx.attr.platforms),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode_indent(manifest, indent = "  ") + "\n")
    return [DefaultInfo(files = depset([out]))]

tested_stack = rule(
    implementation = _tested_stack_impl,
    attrs = {
        "bazel_version": attr.string(
            default = "9.2.0",
            doc = "Must match .bazelversion.",
        ),
        "rules_rust_version": attr.string(
            default = "0.74.0",
            doc = "Must match the rules_rust bazel_dep in MODULE.bazel.",
        ),
        "rules_cc_version": attr.string(
            default = "0.2.22",
            doc = "Must match the rules_cc bazel_dep in MODULE.bazel.",
        ),
        "rust_version": attr.string(
            default = "1.98.0",
            doc = "Must match the rust.toolchain versions in MODULE.bazel.",
        ),
        "platforms": attr.string_list(
            doc = "Tested platform identifiers covered by this manifest.",
        ),
    },
)
