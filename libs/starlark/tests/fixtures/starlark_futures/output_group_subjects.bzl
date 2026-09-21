"""Concrete output-group-subjects use case (issue #794).

Contract: `docs/testing/starlark.md#future-not-implemented`, `docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.

Output-group observation stays deferred: analysis observes `DxSubjectInfo`
fields plus `DefaultInfo` output basenames only, not `OutputGroupInfo`;
wrapper forwarding of output groups does not imply observation. The
group-to-files mapping plus resolved report below stay a Starlark-level
use case. Consumers expose resolved output-group state via `DxSubjectInfo`
when they need it observed.
"""

def admitted_output_groups():
    return ["docs", "artifacts"]

def admitted_output_files():
    return ["guide.md", "bundle.zip"]

def resolve_output_group(group, mapping):
    if group in mapping:
        return mapping[group]
    return "no output group for '" + group + "': want one of " + ", ".join(sorted(mapping.keys()))

def output_group_report(group, files):
    return "Group " + group + " has " + str(len(files)) + " files: " + ", ".join(files) + "!"

def output_group_subject_fields(group, files):
    return {
        "files": ", ".join(files),
        "group": group,
        "report": output_group_report(group, files),
    }

def output_group_fingerprint_like(group):
    return json.encode({
        "files": ["guide.md"],
        "group": group,
    })

def is_supported_output_group(group):
    return group in admitted_output_groups()
