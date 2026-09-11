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

load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

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

# Version of the staged tree management-metadata schema written by
# `environment_tree`. The on-disk `.rules_dx_managed` binary Protobuf
# marker is encoded at install time from this metadata; the schema version
# travels with both so replacement can refuse metadata it cannot validate.
ENV_METADATA_SCHEMA_VERSION = 1

def env_host_filename(name, is_windows):
    """Maps one logical tool name to its host-native filename.

    Args:
      name: validated logical host name (primary or alias).
      is_windows: whether the consuming host requires executable suffixes.

    Returns:
      The logical name unchanged on POSIX, with `.exe` appended on
      Windows. Validation rejects explicit executable suffixes, so the
      mapping never doubles one.
    """
    if is_windows:
        return name + ".exe"
    return name

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

def env_tree_metadata(records, is_windows):
    """Renders the staged tree management metadata as a JSON string.

    Args:
      records: list of `env_tool_record` structs, already validated and
        collision-free by `environment_config`.
      is_windows: whether host filenames carry the Windows suffix.

    Returns:
      JSON with `schema_version` and one entry per tool sorted by
      (`bin_name`, `owner`), each carrying `owner`, `bin_name`, declared
      `aliases`, and mapped `host_names` (primary first). Aliases keep
      declaration order; the encoding is deterministic for a fixed record
      list. The install step encodes the binary `.rules_dx_managed`
      marker from this metadata.
    """
    tools = []
    for record in sorted(records, key = lambda r: (r.bin_name, r.owner)):
        names = [record.bin_name] + list(record.aliases)
        tools.append({
            "aliases": list(record.aliases),
            "bin_name": record.bin_name,
            "host_names": [env_host_filename(name, is_windows) for name in names],
            "owner": record.owner,
        })
    return json.encode({
        "schema_version": ENV_METADATA_SCHEMA_VERSION,
        "tools": tools,
    })

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

    # Persisted labels use the observation rendering so staged bytes stay
    # stable and readable across Bazel renderings (see `display_label`).
    owner = display_label(ctx.label)
    return [
        EnvironmentInfo(
            runners = {owner: ctx.attr.executable.files_to_run},
            tools = depset([env_tool_record(owner, ctx.attr.bin_name, ctx.attr.aliases)]),
        ),
        # The hermetic runtime closure travels through DefaultInfo so
        # configs and trees compose it without touching provider shapes.
        DefaultInfo(
            runfiles = ctx.attr.executable[DefaultInfo].default_runfiles,
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
        DefaultInfo(
            files = depset([]),
            runfiles = ctx.runfiles().merge_all([
                tool[DefaultInfo].default_runfiles
                for tool in ctx.attr.tools
            ]),
        ),
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
            providers = [DefaultInfo, EnvironmentInfo],
        ),
    },
    doc = "Composes environment tool records transitively, failing closed on collisions (M11 WP1).",
)

def _environment_tree_impl(ctx):
    info = ctx.attr.config[EnvironmentInfo]
    records = info.tools.to_list()
    is_windows = ctx.target_platform_has_constraint(
        ctx.attr._windows_os[platform_common.ConstraintValueInfo],
    )
    links = []
    host_names = {}
    for record in records:
        if record.owner not in info.runners:
            fail("environment_tree " + str(ctx.label) + ": no runner for " +
                 record.owner + ": every record needs its environment_tool executable")
        executable = info.runners[record.owner].executable
        for name in [record.bin_name] + list(record.aliases):
            host_name = env_host_filename(name, is_windows)
            host_names[host_name] = True
            link = ctx.actions.declare_file("bin/" + host_name)
            ctx.actions.symlink(
                output = link,
                target_file = executable,
                is_executable = True,
            )
            links.append(link)
    metadata = ctx.actions.declare_file(ctx.label.name + ".metadata.json")
    ctx.actions.write(metadata, env_tree_metadata(records, is_windows))
    runfiles = ctx.runfiles(files = links + [metadata]).merge(
        ctx.attr.config[DefaultInfo].default_runfiles,
    )
    return [
        DefaultInfo(files = depset(links + [metadata]), runfiles = runfiles),
        DxSubjectInfo(fields = {
            "count": str(len(records)),
            "host_names": ",".join(sorted(host_names.keys())),
            "platform": "windows" if is_windows else "posix",
        }),
    ]

environment_tree = rule(
    implementation = _environment_tree_impl,
    attrs = {
        "_windows_os": attr.label(
            default = "@platforms//os:windows",
            doc = "Constraint value detecting Windows target platforms.",
        ),
        "config": attr.label(
            doc = "Validated environment_config whose records stage one symlink per host name.",
            mandatory = True,
            providers = [DefaultInfo, EnvironmentInfo],
        ),
    },
    doc = "Stages the complete symlink-only tool tree plus versioned management metadata (M11 WP2).",
)
