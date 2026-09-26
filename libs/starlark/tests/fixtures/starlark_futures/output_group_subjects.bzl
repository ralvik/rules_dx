"""Concrete output-group-subjects use case.
Output-group observation stays deferred: analysis observes `DxSubjectInfo` fields plus
`DefaultInfo` output basenames only, not `OutputGroupInfo`, so the group-to-files mapping
stays a Starlark-level use case; consumers expose resolved state via `DxSubjectInfo`."""

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
