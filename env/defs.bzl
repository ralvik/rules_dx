"""Bootstrap environment tool registry (M11 WP1).

`environment_tool` validates one host-tool record (a primary `bin_name`
plus `aliases`) against its executable's files to run and exposes
it as `EnvironmentInfo`. `environment_config` composes records
transitively and fails closed on host-name collisions.

Name validation is intentionally conservative: collision keys are always
case-folded, so a config that is collision-free here is collision-free on
case-insensitive hosts (Windows) as well as POSIX hosts. Windows reserved
stems (CON, PRN, AUX, NUL, COM1-9, LPT1-9) and explicit executable suffixes
(.exe, .bat, .cmd, .com) are rejected because suffix materialization owns
the platform suffix (M11 WP2).
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")

EnvironmentInfo = provider(
    doc = "Transitively composed bootstrap environment tool records.",
    fields = {
        "runners": "Dict of owner label string to the tool executable's files-to-run object.",
        "tools": "Depset of tool-record structs (bin_name, aliases tuple, owner).",
    },
)

_HOST_RESERVED_STEMS = (
    "aux",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "con",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
    "nul",
    "prn",
)

_EXECUTABLE_SUFFIXES = (".bat", ".cmd", ".com", ".exe")

def env_name_error(name):
    """Validates one host command name.

    Args:
      name: candidate host command name (primary or alias).

    Returns:
      "" when valid, else the failure reason: empty, dot segment, path
      separator, executable suffix, or Windows reserved stem.
    """
    if name == "":
        return "invalid host name '': must be a non-empty single path component"
    if name == "." or name == "..":
        return "invalid host name '" + name + "': must not be '.' or '..'"
    if "/" in name or "\\" in name:
        return "invalid host name '" + name + "': must not contain '/' or '\\'"
    lower = name.lower()
    for suffix in _EXECUTABLE_SUFFIXES:
        if lower.endswith(suffix):
            return "invalid host name '" + name + "': must not carry an executable suffix (found '" + suffix + "')"
    stem = lower.split(".")[0]
    if stem in _HOST_RESERVED_STEMS:
        return "invalid host name '" + name + "': stem '" + stem + "' is reserved on Windows"
    return ""

def env_tool_error(bin_name, aliases):
    """Validates one tool record.

    Args:
      bin_name: primary host command name.
      aliases: additional host command names for the same executable.

    Returns:
      "" when valid, else the failure reason naming the bad primary or alias.
    """
    primary_error = env_name_error(bin_name)
    if primary_error != "":
        return "invalid bin_name: " + primary_error
    for alias in aliases:
        alias_error = env_name_error(alias)
        if alias_error != "":
            return "invalid alias '" + alias + "': " + alias_error
    return ""

def env_tool_record(owner, bin_name, aliases):
    """Builds one tool-record struct for collision analysis.

    Args:
      owner: label string of the claiming `environment_tool`.
      bin_name: primary host command name.
      aliases: additional host command names for the same executable.

    Returns:
      A struct with `owner`, `bin_name`, and `aliases` (as a tuple).
    """
    return struct(
        aliases = tuple(aliases),
        bin_name = bin_name,
        owner = owner,
    )

def env_collision_error(records):
    """Detects host-name collisions across tool records.

    Every claimed name (primary plus aliases) is keyed case-folded; any key
    claimed by more than one owner fails, listing every claimant. Repeated
    claims by a single owner (for example an alias equal to its bin_name)
    are one claim, not a collision.

    Args:
      records: list of `env_tool_record` structs.

    Returns:
      "" when collision-free, else the failure reason listing every
      collided host name with its sorted claimant owners.
    """
    owners_by_key = {}
    for record in records:
        seen = {}
        for name in [record.bin_name] + list(record.aliases):
            key = name.lower()
            if key in seen:
                continue
            seen[key] = True
            if key not in owners_by_key:
                owners_by_key[key] = {}
            owners_by_key[key][record.owner] = True
    collisions = []
    for key in sorted(owners_by_key.keys()):
        owners = sorted(owners_by_key[key].keys())
        if len(owners) > 1:
            collisions.append(
                "host name '" + key + "' claimed by " + ", ".join(owners),
            )
    if not collisions:
        return ""
    return "host-name collision: " + "; ".join(collisions)

def _environment_tool_impl(ctx):
    error = env_tool_error(ctx.attr.bin_name, ctx.attr.aliases)
    if error != "":
        fail("environment_tool " + str(ctx.label) + ": " + error)
    owner = str(ctx.label)
    return [
        EnvironmentInfo(
            runners = {owner: ctx.attr.executable.files_to_run},
            tools = depset([env_tool_record(owner, ctx.attr.bin_name, ctx.attr.aliases)]),
        ),
    ]

environment_tool = rule(
    implementation = _environment_tool_impl,
    attrs = {
        "aliases": attr.string_list(
            default = [],
            doc = "Additional host command names served by the same executable.",
        ),
        "bin_name": attr.string(
            mandatory = True,
            doc = "Primary host command name for this tool.",
        ),
        "executable": attr.label(
            cfg = "exec",
            executable = True,
            mandatory = True,
            doc = "Host executable backing every claimed host name.",
        ),
    },
    doc = "Validates one bootstrap environment tool record (M11 WP1).",
)

def _environment_config_impl(ctx):
    records = []
    runners = {}
    for tool in ctx.attr.tools:
        info = tool[EnvironmentInfo]
        records.extend(info.tools.to_list())
        runners.update(info.runners)
    error = env_collision_error(records)
    if error != "":
        fail("environment_config " + str(ctx.label) + ": " + error)
    names = {}
    for record in records:
        names[record.bin_name] = True
        for alias in record.aliases:
            names[alias] = True
    return [
        EnvironmentInfo(
            runners = runners,
            tools = depset(records),
        ),
        DefaultInfo(files = depset([])),
        DxSubjectInfo(fields = {
            "count": str(len(records)),
            "names": ",".join(sorted(names.keys())),
        }),
    ]

environment_config = rule(
    implementation = _environment_config_impl,
    attrs = {
        "tools": attr.label_list(
            default = [],
            doc = "environment_tool and environment_config targets composed transitively.",
            providers = [EnvironmentInfo],
        ),
    },
    doc = "Composes environment tool records transitively, failing closed on collisions (M11 WP1).",
)
