"""JVM tool acquisition via a lazy module extension.

One lazy repo per tool; registration fetches nothing (no per-platform matrix: JVM tools
run over the shared JDK).
"""

load("@bazel_tools//tools/build_defs/repo:http.bzl", _http_archive = "http_archive", _http_file = "http_file")
load("//quality/tools/jvm:repos.bzl", "JVM_TOOLS")

def _jvm_tools_impl(_ctx):
    for name in sorted(JVM_TOOLS.keys()):
        spec = JVM_TOOLS[name]
        if spec["kind"] == "file":
            _http_file(
                name = name,
                downloaded_file_path = spec["downloaded_file_path"],
                sha256 = spec["sha256"],
                urls = spec["urls"],
            )
        elif spec["kind"] == "archive":
            _http_archive(
                name = name,
                build_file = spec["build_file"],
                sha256 = spec["sha256"],
                strip_prefix = spec["strip_prefix"],
                urls = spec["urls"],
            )
        else:
            fail("unsupported JVM tool kind for '" + name + "': " + spec["kind"])

jvm_tools = module_extension(implementation = _jvm_tools_impl)
