"""Tested-stack manifest generator (ADR 0014).

Contract: `docs/decisions/0014-tested-platform-release-stack.md`.
"""

# Regenerate the defaults below from the executed module graph (single source
# for the preset pins, `.bazelversion`, and the support matrix):
#   bazel mod deps --depth=1 --format=json | python3 -c \
#     'import json,sys; [print(m["name"], m["version"]) for m in json.load(sys.stdin)["modules"]]'
# Every default must equal its MODULE.bazel pin; `//tools/ci:pin_consistency_test`
# fails on drift, and `//libs/testing:stack_contract` pins the manifest shape.
_TESTED_DEPS = {
    "rules_rust": "0.74.0",
    "rules_cc": "0.2.22",
    "googletest": "1.18.0",
    "rules_python": "1.9.0",
    "rules_go": "0.63.0",
    "gazelle": "0.52.2",
    "rules_shell": "0.6.1",
    "platforms": "1.1.0",
    "bazel_skylib": "1.9.0",
    "rules_proto": "7.1.0",
    "rules_rust_prost": "0.74.0",
    "rules_java": "9.7.0",
    "rules_kotlin": "2.4.10",
    "rules_scala": "7.3.0",
    "rules_dotnet": "0.22.1",
    "bazel_lib": "3.7.0",
    "rules_jvm_external": "7.1",
    "aspect_rules_py": "2.0.0-alpha.6",
    "aspect_rules_js": "3.4.1",
    "aspect_rules_ts": "3.10.0",
    "aspect_rules_jest": "0.26.0",
}

def _tested_stack_impl(ctx):
    deps = dict(ctx.attr.deps)
    for name, version in _TESTED_DEPS.items():
        deps.setdefault(name, version)
    manifest = {
        "bazel_version": ctx.attr.bazel_version,
        "dotnet_version": ctx.attr.dotnet_version,
        "generator": "libs/testing/tested_stack.bzl",
        "go_sdk_version": ctx.attr.go_sdk_version,
        "module_deps": deps,
        "platforms": sorted(ctx.attr.platforms),
        "pnpm_version": ctx.attr.pnpm_version,
        "python_version": ctx.attr.python_version,
        "rules_cc_version": ctx.attr.rules_cc_version,
        "rules_rust_version": ctx.attr.rules_rust_version,
        "rust_version": ctx.attr.rust_version,
        "scala_version": ctx.attr.scala_version,
        "schema_version": 2,
        "typescript_version": ctx.attr.typescript_version,
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
        "deps": attr.string_dict(
            default = {},
            doc = "Module dep overrides; defaults cover every direct MODULE.bazel bazel_dep.",
        ),
        "dotnet_version": attr.string(
            default = "10.0.201",
            doc = "Must match the dotnet.toolchain dotnet_version in MODULE.bazel.",
        ),
        "go_sdk_version": attr.string(
            default = "1.26.6",
            doc = "Must match the go_sdk.download version in MODULE.bazel.",
        ),
        "platforms": attr.string_list(
            doc = "Tested platform identifiers covered by this manifest.",
        ),
        "pnpm_version": attr.string(
            default = "10.34.5",
            doc = "Must match the root package.json packageManager pin.",
        ),
        "python_version": attr.string(
            default = "3.12",
            doc = "Must match the python.toolchain python_version in MODULE.bazel.",
        ),
        "rules_cc_version": attr.string(
            default = "0.2.22",
            doc = "Must match the rules_cc bazel_dep in MODULE.bazel.",
        ),
        "rules_rust_version": attr.string(
            default = "0.74.0",
            doc = "Must match the rules_rust bazel_dep in MODULE.bazel.",
        ),
        "rust_version": attr.string(
            default = "1.98.0",
            doc = "Must match the rust.toolchain versions in MODULE.bazel.",
        ),
        "scala_version": attr.string(
            default = "2.13.18",
            doc = "Must match the scala_config.settings scala_version in MODULE.bazel.",
        ),
        "typescript_version": attr.string(
            default = "5.9.3",
            doc = "Must match the typescript.deps version in MODULE.bazel.",
        ),
    },
)
