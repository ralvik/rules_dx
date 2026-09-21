"""Concrete toolchain-subjects use case (issue #792).

Contract: `docs/testing/starlark.md#future-not-implemented`, `docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.

Toolchain resolution needs platform plus toolchain context beyond
provider-field observation: analysis observes `DxSubjectInfo` fields plus
`DefaultInfo` output basenames only, so the platform-to-toolchain mapping
plus the resolved report below stay a Starlark-level use case. Direct
toolchain observation stays deferred; consumers expose resolved toolchain
state via `DxSubjectInfo` when they need it observed.
"""

def admitted_platforms():
    return ["linux_x86_64", "macos_arm64"]

def admitted_toolchains():
    return ["gcc", "clang"]

def resolve_toolchain(platform, mapping):
    if platform in mapping:
        return mapping[platform]
    return "no toolchain for '" + platform + "': want one of " + ", ".join(sorted(mapping.keys()))

def toolchain_report(platform, toolchain):
    return "Hello from " + toolchain + " on " + platform + "!"

def toolchain_subject_fields(platform, toolchain):
    return {
        "platform": platform,
        "report": toolchain_report(platform, toolchain),
        "toolchain": toolchain,
    }

def toolchain_fingerprint_like(toolchain):
    return json.encode({
        "platform": "linux_x86_64",
        "toolchain": toolchain,
    })

def is_supported_platform(platform):
    return platform in admitted_platforms()
