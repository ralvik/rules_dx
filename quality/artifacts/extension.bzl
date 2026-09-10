"""M04 standalone quality-tool acquisition (WP1, O20).

One repository per tool/platform is materialized from the generated
checked-in metadata in this directory; a hub repository selects the
execution-platform artifact. Registration never fetches: Bazel downloads a
tool repository only when an action needs its artifact, and `ctx.download`
rejects bytes whose digest differs from the checked-in pin.

Metadata schema version 1 is frozen by `metadata_tests.bzl`; only
linux_x86_64 is recorded (Linux-first scope, other platforms are gaps).
"""

load("//quality/artifacts:buildifier.linux_x86_64.bzl", _buildifier_linux_x86_64 = "ARTIFACT")
load("//quality/artifacts:taplo.linux_x86_64.bzl", _taplo_linux_x86_64 = "ARTIFACT")
load("//quality/artifacts:vale.linux_x86_64.bzl", _vale_linux_x86_64 = "ARTIFACT")

_ARTIFACTS = [
    _buildifier_linux_x86_64,
    _taplo_linux_x86_64,
    _vale_linux_x86_64,
]

def _repo_name(artifact):
    return "dx_%s_%s_%s" % (artifact["tool"], artifact["os"], artifact["cpu"])

def _standalone_tool_repo_impl(ctx):
    kind = ctx.attr.archive_format
    if kind == "none":
        ctx.download(
            url = ctx.attr.url,
            output = ctx.attr.executable,
            sha256 = ctx.attr.sha256,
        )
    elif kind == "gzip":
        ctx.download(
            url = ctx.attr.url,
            output = ctx.attr.asset,
            sha256 = ctx.attr.sha256,
        )

        # A bare single-file gzip extracts to the repo root under its
        # recorded member name; no output directory is used.
        ctx.extract(ctx.attr.asset)
    elif kind == "tar.gz":
        ctx.download(
            url = ctx.attr.url,
            output = ctx.attr.asset,
            sha256 = ctx.attr.sha256,
        )
        ctx.extract(ctx.attr.asset)
    else:
        fail("unsupported archive format: " + kind)
    result = ctx.execute(["chmod", "0755", ctx.attr.executable])
    if result.return_code != 0:
        fail("chmod failed for " + ctx.attr.executable + ": " + result.stderr)
    ctx.file("BUILD.bazel", "\n".join([
        "filegroup(",
        '    name = "tool",',
        "    srcs = [%r]," % ctx.attr.executable,
        '    visibility = ["//visibility:public"],',
        ")",
        "",
    ]))

_standalone_tool_repo = repository_rule(
    implementation = _standalone_tool_repo_impl,
    attrs = {
        "archive_format": attr.string(mandatory = True),
        "asset": attr.string(mandatory = True),
        "executable": attr.string(mandatory = True),
        "sha256": attr.string(mandatory = True),
        "url": attr.string(mandatory = True),
    },
)

_HUB_BUILD = """config_setting(
    name = "linux_x86_64",
    constraint_values = [
        "{os_linux}",
        "{cpu_x86_64}",
    ],
)
"""

_HUB_ALIAS = """alias(
    name = "{tool}",
    actual = select(
        {{
            ":linux_x86_64": "@{repo}//:tool",
        }},
        no_match_error = (
            "rules_dx: no {tool} artifact for this execution platform; " +
            "only linux_x86_64 is recorded (see //quality/artifacts). " +
            "Other platforms are unimplemented gaps, not silent fallbacks."
        ),
    ),
    visibility = ["//visibility:public"],
)
"""

def _hub_repo_impl(ctx):
    lines = [_HUB_BUILD.format(
        os_linux = ctx.attr.os_linux,
        cpu_x86_64 = ctx.attr.cpu_x86_64,
    )]
    for tool, repos in ctx.attr.artifacts.items():
        lines.append(_HUB_ALIAS.format(tool = tool, repo = repos[0]))
    ctx.file("BUILD.bazel", "\n".join(lines))

_hub_repo = repository_rule(
    implementation = _hub_repo_impl,
    attrs = {
        "artifacts": attr.string_list_dict(mandatory = True),
        "cpu_x86_64": attr.string(mandatory = True),
        # Canonical platform labels supplied by the MODULE.bazel tag, so the
        # generated BUILD references repositories visible from the hub.
        "os_linux": attr.string(mandatory = True),
    },
)

def _dx_tools_impl(ctx):
    os_linux, cpu_x86_64 = None, None
    for module in ctx.modules:
        for tag in module.tags.platform:
            if os_linux != None:
                fail("dx_tools.platform may be declared at most once")
            os_linux, cpu_x86_64 = str(tag.os_linux), str(tag.cpu_x86_64)
    if os_linux == None:
        fail("dx_tools.platform(os_linux, cpu_x86_64) is required in MODULE.bazel")
    hub_entries = {}
    for artifact in _ARTIFACTS:
        name = _repo_name(artifact)
        _standalone_tool_repo(
            name = name,
            url = artifact["url"],
            sha256 = artifact["sha256"],
            asset = artifact["url"].split("/")[-1],
            archive_format = artifact["archive"]["format"],
            executable = artifact["executable"],
        )
        hub_entries[artifact["tool"]] = [name]
    _hub_repo(
        name = "dx_tools",
        artifacts = hub_entries,
        os_linux = os_linux,
        cpu_x86_64 = cpu_x86_64,
    )

_platform = tag_class(attrs = {
    "cpu_x86_64": attr.label(mandatory = True),
    "os_linux": attr.label(mandatory = True),
})

dx_tools = module_extension(
    implementation = _dx_tools_impl,
    tag_classes = {"platform": _platform},
)
