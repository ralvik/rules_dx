"""Load tests pinning standalone-artifact metadata (M04 WP1, O20).

The schema key set is frozen: adding, removing, or renaming a field fails
until reviewed. Upstream versions, URLs, digests, and sizes are pinned per
tool so a re-pin (update or changed upstream bytes) is an explicit reviewed
edit. `bazel run //quality/artifacts:update -- --verify-only` reproduces these
files from the network; this test guards the checked-in content hermetically.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":buildifier.linux_x86_64.bzl", _buildifier = "ARTIFACT")
load(":taplo.linux_x86_64.bzl", _taplo = "ARTIFACT")
load(":vale.linux_x86_64.bzl", _vale = "ARTIFACT")

# Frozen schema surface (M04 WP1): sorted ARTIFACT keys.
FROZEN_SCHEMA_KEYS = [
    "abi_floor",
    "archive",
    "checksum_source",
    "cpu",
    "delivery_class",
    "executable",
    "executable_sha256",
    "interpreter",
    "licenses",
    "linkage",
    "needed_shared_libraries",
    "os",
    "runtime_files",
    "schema_version",
    "sha256",
    "size",
    "tool",
    "upstream_version",
    "url",
]

def _artifact_checks(artifact, tool, version, url, sha256, size, exe, exe_sha256):
    return [
        expect_equal(tool + " schema keys", sorted(artifact.keys()), FROZEN_SCHEMA_KEYS),
        expect_equal(tool + " schema version", artifact["schema_version"], 1),
        expect_equal(tool + " delivery class", artifact["delivery_class"], "standalone"),
        expect_equal(tool + " upstream version", artifact["upstream_version"], version),
        expect_equal(tool + " url", artifact["url"], url),
        expect_equal(tool + " sha256", artifact["sha256"], sha256),
        expect_equal(tool + " size", artifact["size"], size),
        expect_equal(tool + " executable", artifact["executable"], exe),
        expect_equal(tool + " executable sha256", artifact["executable_sha256"], exe_sha256),
        expect_equal(tool + " platform", artifact["os"] + "_" + artifact["cpu"], "linux_x86_64"),
    ]

def metadata_tests(name):
    """Declare the standalone-artifact metadata pin test.

    Args:
      name: test target name.
    """
    checks = []
    checks += _artifact_checks(
        _buildifier,
        "buildifier",
        "8.5.1",
        "https://github.com/bazel-contrib/buildtools/releases/download/v8.5.1/buildifier-linux-amd64",
        "887377fc64d23a850f4d18a077b5db05b19913f4b99b270d193f3c7334b5a9a7",
        7757842,
        "buildifier-linux-amd64",
        "887377fc64d23a850f4d18a077b5db05b19913f4b99b270d193f3c7334b5a9a7",
    )
    checks += _artifact_checks(
        _taplo,
        "taplo",
        "0.10.0",
        "https://github.com/tamasfe/taplo/releases/download/0.10.0/taplo-linux-x86_64.gz",
        "8fe196b894ccf9072f98d4e1013a180306e17d244830b03986ee5e8eabeb6156",
        5116068,
        "taplo-x86_64",
        "dad2faf6377d2daa4f4fabf459fe7ccfb98a5448f0d4bca8270ca9acb0409bfe",
    )
    checks += _artifact_checks(
        _vale,
        "vale",
        "3.20.0",
        "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_Linux_64-bit.tar.gz",
        "f59e7030c5d4ace6cf915497d0d076a1699d61e876142765963237e6867c9712",
        11736086,
        "vale",
        "17bd39892f5c2e2f62c72d85be47facd83c21d39597154996134d15e34af4428",
    )
    starlark_test(
        name = name,
        mode = "load",
        checks = checks,
    )
