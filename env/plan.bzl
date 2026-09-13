"""Normalized environment plan records (M25 WP2).

`DxEnvPlanInfo` is the in-memory shape behind the binary `DxEnvShard`
wire schema (`plan.proto`): each shard-emitting rule carries its direct
environment identity record. `DxEnvPlanCollectedInfo` is the separate
aspect-carried merge: Bazel rejects an aspect that re-provides its
target's own provider ("provided twice"), so the collecting aspect never
returns `DxEnvPlanInfo` and contributing rules never return the
collected provider. Shard emission, the collecting aspect, and the
language adapters land in the next WP2 slice (not this file).

First-integration freeze (slice 1): the only first-release language
integration is Rust, the first language-native projection used by the
repository (`docs/environments/environment.md`). Python's `.venv` is a
later integration rather than the generic model; Node and future
integrations stay deferred to later M25 slices. They are not dropped
and no adapter claims them here.

Conflict rule: multiple producers claiming one identity key with
different values or exec paths fail before selection; no
traversal-order winner is accepted. Byte-identical duplicate records
(the same producer, integration, key, value, and exec path) merge
silently, since transitive collection reaches one record through many
routes.

Each entry carries an optional `exec_path` BEP-matching suffix: empty
means a logical-only identity input requiring no materialized artifact;
non-empty must resolve to exactly one BEP-reported non-shard artifact
(suffix match so output bases differ) and every non-shard BEP artifact
must be claimed, otherwise collection fails before projection planning.
The fingerprint covers the exec path, so the plan identity binds
identity inputs to their backing artifacts.

`EnvironmentInfo` remains PATH-tool-only and is not this internal plan
contract (`docs/environments/environment.md`).

Contract: `docs/environments/environment.md` (plan collection,
provider-selective aspects, private output group), `docs/environments
/managed-state.md` (identity, selection, carry-forward).

Slice 2 (below) adds the shard-emission rule, the collecting aspect,
and the narrow Rust adapter over the frozen slice-1 semantics. No new
integration, key, or merge semantic lands here.
"""

load("@rules_rust//rust:defs.bzl", _rust_common = "rust_common")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

DxEnvPlanInfo = provider(
    doc = "Normalized environment plan records: direct plus transitive collection.",
    fields = {
        "direct": "List of environment record structs contributed by this target.",
        "transitive": "Depset of environment record structs from the closure.",
    },
)

# Aspect-carried merge over DxEnvPlanInfo contributors. Kept distinct
# because Bazel reports "provided twice" when an aspect returns the same
# provider its target already provides: shard rules carry direct records
# in DxEnvPlanInfo, the aspect merges them here, and fixtures prefer
# this provider with a direct-rule fallback.
DxEnvPlanCollectedInfo = provider(
    doc = "Aspect-merged environment plan records from the traversed closure.",
    fields = {
        "records": "Depset of merged environment record structs.",
    },
)

# Private output group carrying collected shards plus every referenced
# artifact. Frozen here; the aspect requests it.
DX_ENV_PLAN_OUTPUT_GROUP = "dx_env_plans"

# Reserved controlled filename suffix recognizing shards among
# BEP-reported files. Frozen here; the CLI rejects unreported,
# missing, or duplicate artifacts and never scans `bazel-out`.
DX_ENV_SHARD_SUFFIX = ".dxenv.pb"

# Admitted first-release language integrations, slice 1. Later M25
# slices extend this tuple only through recorded qualification: Rust
# first, then the deferred Python/Node/future integrations.
DX_ENV_ADMITTED_INTEGRATIONS = (
    "rust",
)

def env_plan_key_error(key):
    """Validates one identity-dimension key.

    Args:
      key: candidate identity dimension, e.g. "runtime" or "abi".

    Returns:
      "" when valid, else the failure reason: empty, a path separator,
      or the shard-entry "|" separator.
    """
    if key == "":
        return "invalid env plan key '': must be a non-empty single token"
    if "/" in key or "\\" in key:
        return "invalid env plan key '" + key + "': must not contain '/' or '\\'"
    if "|" in key:
        return "invalid env plan key '" + key + "': must not contain '|'"
    return ""

def env_plan_value_error(value):
    """Validates one identity-input value.

    Args:
      value: candidate normalized identity input value.

    Returns:
      "" when valid, else the failure reason: empty or carrying the
      shard-entry "|" separator.
    """
    if value == "":
        return "invalid env plan value '': must be a non-empty identity input"
    if "|" in value:
        return "invalid env plan value '" + value + "': must not contain '|'"
    return ""

def env_plan_exec_error(path):
    """Validates one BEP-matching exec-path suffix.

    Empty means a logical-only identity input requiring no artifact.
    Non-empty must be a workspace-relative path and never uses the
    reserved shard suffix (a shard never backs another shard).

    Args:
      path: candidate exec-path suffix.

    Returns:
      "" when valid, else the failure reason.
    """
    if path == "":
        return ""
    if path.endswith(DX_ENV_SHARD_SUFFIX):
        return "invalid env plan exec path '" + path + "': must not use the reserved shard suffix '" + DX_ENV_SHARD_SUFFIX + "'"
    if path.startswith("/"):
        return "invalid env plan exec path '" + path + "': must not be absolute"
    if "\\" in path:
        return "invalid env plan exec path '" + path + "': must not contain '\\'"
    for segment in path.split("/"):
        if segment == "." or segment == "..":
            return "invalid env plan exec path '" + path + "': must not contain '.' or '..' segments"
    return ""

def env_plan_entry(key, value, exec_path = ""):
    """Builds one normalized environment identity entry struct.

    Args:
      key: normalized identity dimension, e.g. "runtime".
      value: normalized identity input value for the key.
      exec_path: BEP-matching suffix for the backing artifact, or "" for
        a logical-only identity input requiring no materialized artifact.

    Returns:
      A struct with `key`, `value`, and `exec_path`.
    """
    return struct(
        exec_path = exec_path,
        key = key,
        value = value,
    )

def env_plan_record(producer, integration, entries):
    """Builds one normalized contributor record struct.

    Args:
      producer: contributing target label in observation rendering.
      integration: language integration class, e.g. "rust".
      entries: non-empty list of `env_plan_entry` structs.

    Returns:
      A struct with `producer`, `integration`, and `entries` (as a tuple).
    """
    return struct(
        entries = tuple(entries),
        integration = integration,
        producer = producer,
    )

def env_plan_record_error(record):
    """Validates one contributor record.

    Args:
      record: candidate `env_plan_record` struct.

    Returns:
      "" when valid, else the failure reason naming the bad producer,
      integration, entry key/value, or within-record duplicate key.
    """
    if record.producer == "":
        return "invalid env plan record: producer must be a non-empty label"
    if not (record.producer.startswith("//") or record.producer.startswith("@")):
        return "invalid env plan record '" + record.producer + "': producer must be a label in observation rendering"
    if record.integration == "":
        return "invalid env plan record '" + record.producer + "': integration must be a non-empty language class"
    if len(record.entries) == 0:
        return "invalid env plan record '" + record.producer + "': entries must be non-empty (targets with no contribution carry no record)"
    seen = {}
    for entry in record.entries:
        error = env_plan_key_error(entry.key)
        if error != "":
            return "invalid env plan record '" + record.producer + "': " + error
        error = env_plan_value_error(entry.value)
        if error != "":
            return "invalid env plan record '" + record.producer + "': " + error
        error = env_plan_exec_error(entry.exec_path)
        if error != "":
            return "invalid env plan record '" + record.producer + "': " + error
        if entry.key in seen:
            return "invalid env plan record '" + record.producer + "': duplicate key '" + entry.key + "'"
        seen[entry.key] = True
    return ""

def _env_plan_entry_key(entry):
    return (entry.key, entry.value, entry.exec_path)

def env_plan_conflict_error(records):
    """Detects incompatible identity-key claims across records.

    Byte-identical duplicates (same producer, integration, key, value,
    and exec path) merge silently. Any other second claim on one key
    fails, listing every claimant: no traversal-order winner is accepted.

    Args:
      records: list of `env_plan_record` structs.

    Returns:
      "" when conflict-free, else the failure reason listing every
      collided key with its sorted claimant producers.
    """
    claimants_by_key = {}
    for record in records:
        for entry in record.entries:
            key = (record.producer, record.integration) + _env_plan_entry_key(entry)
            keys = claimants_by_key.setdefault(entry.key, {})
            keys[key] = True
    conflicts = []
    for key in sorted(claimants_by_key.keys()):
        claims = sorted(claimants_by_key[key].keys())
        if len(claims) > 1:
            conflicts.append(
                "identity key '" + key + "' claimed by " +
                ", ".join([claim[0] for claim in claims]),
            )
    if not conflicts:
        return ""
    return "env plan conflict: " + "; ".join(conflicts)

def env_plan_merge_records(records):
    """Merges records into deterministic normalized order.

    Byte-identical duplicate entries collapse; surviving records sort
    by (producer, integration) with entries sorted by (key, value, exec
    path). The rendering is the normalized complete-plan form the CLI
    hashes.

    Args:
      records: list of `env_plan_record` structs.

    Returns:
      The deduplicated record list in normalized order.
    """
    entries_by_owner = {}
    for record in records:
        owner = (record.producer, record.integration)
        owned = entries_by_owner.setdefault(owner, {})
        for entry in record.entries:
            owned[_env_plan_entry_key(entry)] = entry
    merged = []
    for owner in sorted(entries_by_owner.keys()):
        entries = [
            entries_by_owner[owner][key]
            for key in sorted(entries_by_owner[owner].keys())
        ]
        merged.append(env_plan_record(owner[0], owner[1], entries))
    return merged

def env_plan_fingerprint(records):
    """Renders the normalized complete-plan hash input.

    Args:
      records: list of `env_plan_record` structs.

    Returns:
      Deterministic JSON over the merged records: one object per record
      with producer, integration, and entries sorted by (key, value,
      exec path), each entry carrying its exec-path suffix so the
      identity binds inputs to artifacts.
    """
    merged = env_plan_merge_records(records)
    return json.encode([
        {
            "entries": [
                {
                    "exec_path": entry.exec_path,
                    "key": entry.key,
                    "value": entry.value,
                }
                for entry in record.entries
            ],
            "integration": record.integration,
            "producer": record.producer,
        }
        for record in merged
    ])

def env_plan_integration_error(integration):
    """Validates one language integration against the slice-1 freeze.

    Args:
      integration: language integration class, e.g. "rust".

    Returns:
      "" when the integration is admitted, else the failure reason.
    """
    if integration in DX_ENV_ADMITTED_INTEGRATIONS:
        return ""
    return (
        "unsupported env plan integration '" + integration + "': " +
        "admitted first-release integrations are " + str(DX_ENV_ADMITTED_INTEGRATIONS)
    )

def _parse_entry_spec(spec, label_text):
    """Parses one KEY|VALUE[|EXEC] entry spec.

    The two-part form declares a logical-only identity input (empty exec
    path, requiring no materialized artifact). The three-part form
    declares the BEP-matching exec-path suffix for the backing artifact.

    Args:
      spec: raw entry string with one or two "|" separators.
      label_text: owning label rendering for diagnostics.

    Returns:
      An `env_plan_entry` struct.
    """
    parts = spec.split("|")
    if len(parts) == 2:
        return env_plan_entry(parts[0], parts[1])
    if len(parts) == 3:
        return env_plan_entry(parts[0], parts[1], parts[2])
    fail(
        "dx_env_shard " + label_text +
        ": bad entry " + repr(spec) +
        ": want KEY|VALUE[|EXEC_PATH]",
    )

def _emit_shard(ctx, producer, integration, entry_structs):
    """Validates one record and emits its binary shard via the writer.

    Args:
      ctx: rule context with executable `_writer`.
      producer: contributor label in observation rendering.
      integration: language integration class.
      entry_structs: list of `env_plan_entry` structs.

    Returns:
      The declared shard file and the validated record struct.
    """
    record = env_plan_record(producer, integration, entry_structs)
    record_error = env_plan_record_error(record)
    if record_error != "":
        fail("dx_env_shard " + producer + ": " + record_error)
    out = ctx.actions.declare_file(ctx.label.name + DX_ENV_SHARD_SUFFIX)
    args = ctx.actions.args()
    args.add("--producer", producer)
    args.add("--integration", integration)
    for entry in entry_structs:
        if entry.exec_path == "":
            args.add(
                "--entry",
                entry.key + "|" + entry.value,
            )
        else:
            args.add(
                "--entry",
                entry.key + "|" + entry.value + "|" + entry.exec_path,
            )
    args.add("--output", out.path)
    ctx.actions.run(
        executable = ctx.executable._writer,
        arguments = [args],
        outputs = [out],
        mnemonic = "DxEnvShard",
        progress_message = "Dx env shard %{label}",
    )
    return out, record

def _exec_matches(file_path, exec_path):
    """Reports whether a Bazel file path satisfies an exec-path suffix.

    Suffix matching (on "/" boundaries, plus exact equality) lets one
    identity input resolve under different output bases without scanning
    `bazel-out`.

    Args:
      file_path: Bazel `File.path` of a backing artifact.
      exec_path: non-empty BEP-matching suffix from an entry.

    Returns:
      True when `file_path` equals `exec_path` or ends with
      `"/" + exec_path`.
    """
    return file_path == exec_path or file_path.endswith("/" + exec_path)

def _dx_env_shard_impl(ctx):
    producer = display_label(ctx.label)
    integration_error = env_plan_integration_error(ctx.attr.integration)
    if integration_error != "":
        fail("dx_env_shard " + producer + ": " + integration_error)
    entries = [_parse_entry_spec(spec, producer) for spec in ctx.attr.entries]
    out, record = _emit_shard(ctx, producer, ctx.attr.integration, entries)
    return [
        DefaultInfo(files = depset([out])),
        DxEnvPlanInfo(direct = [record], transitive = depset([record])),
        OutputGroupInfo(dx_env_plans = depset([out])),
    ]

dx_env_shard = rule(
    implementation = _dx_env_shard_impl,
    attrs = {
        "deps": attr.label_list(
            default = [],
            doc = "Graph edges the collecting aspect traverses; contributes no records itself.",
        ),
        "entries": attr.string_list(
            mandatory = True,
            doc = "Non-empty identity entries, each KEY|VALUE[|EXEC_PATH].",
        ),
        "integration": attr.string(
            default = "rust",
            doc = "Language integration class, e.g. 'rust'. Only the slice-1 freeze is admitted.",
        ),
        "_writer": attr.label(
            default = "//env/env_shard:env_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxEnvShard protobuf.",
        ),
    },
    doc = "Emits one contributor's normalized binary environment plan shard (M25 WP2).",
)

def _edge_targets(rule_attr, name):
    value = getattr(rule_attr, name, [])
    if value == None:
        return []
    if type(value) == "Target":
        return [value]
    return value

# Narrow traversal edges for the collecting aspect: the shard rule's own
# `deps` and the Rust adapter's `target` edge. No `data`, `srcs`,
# `DefaultInfo`, or broad unions are traversed: collectors consume only
# normalized providers.
_ENV_PLAN_ASPECT_ATTRS = ["deps", "target"]

def _dx_env_plan_aspect_impl(target, ctx):
    direct_records = []
    direct_files = []
    if DxEnvPlanInfo in target:
        direct_records = target[DxEnvPlanInfo].direct
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if DX_ENV_PLAN_OUTPUT_GROUP in groups:
            direct_files = groups[DX_ENV_PLAN_OUTPUT_GROUP].to_list()
    transitive_records = []
    transitive_files = []
    for name in _ENV_PLAN_ASPECT_ATTRS:
        for dep in _edge_targets(ctx.rule.attr, name):
            if DxEnvPlanCollectedInfo in dep:
                transitive_records.append(dep[DxEnvPlanCollectedInfo].records)
            if OutputGroupInfo in dep:
                groups = dep[OutputGroupInfo]
                if DX_ENV_PLAN_OUTPUT_GROUP in groups:
                    transitive_files.append(groups[DX_ENV_PLAN_OUTPUT_GROUP])
    merged_records = depset(direct_records, transitive = transitive_records)
    conflict = env_plan_conflict_error(merged_records.to_list())
    if conflict != "":
        fail("dx_env_plan_aspect on " + display_label(target.label) + ": " + conflict)
    return [
        DxEnvPlanCollectedInfo(records = merged_records),
        OutputGroupInfo(dx_env_plans = depset(direct_files, transitive = transitive_files)),
    ]

dx_env_plan_aspect = aspect(
    implementation = _dx_env_plan_aspect_impl,
    attr_aspects = _ENV_PLAN_ASPECT_ATTRS,
    doc = "Collects normalized environment plan records and shard files along narrow env edges.",
)

def _rust_env_shard_impl(ctx):
    producer = display_label(ctx.label)
    integration_error = env_plan_integration_error(ctx.attr.integration)
    if integration_error != "":
        fail("rust_env_shard " + producer + ": " + integration_error)
    upstream = ctx.attr.target
    if _rust_common.crate_info in upstream:
        crate = upstream[_rust_common.crate_info]
    elif _rust_common.test_crate_info in upstream:
        crate = upstream[_rust_common.test_crate_info].crate
    else:
        fail(
            "rust_env_shard " + producer + ": upstream " +
            display_label(upstream.label) +
            " carries neither CrateInfo nor TestCrateInfo (not a dx_rust_* wrapper)",
        )
    candidates_by_path = {}
    for f in crate.srcs.to_list() + [crate.root]:
        candidates_by_path[f.path] = f
    candidates = sorted(candidates_by_path.values(), key = lambda f: f.path)
    entries = [_parse_entry_spec(spec, producer) for spec in ctx.attr.entries]
    for entry in entries:
        if entry.exec_path == "":
            continue
        matches = [f for f in candidates if _exec_matches(f.path, entry.exec_path)]
        if len(matches) == 0:
            fail(
                "rust_env_shard " + producer + ": entry '" +
                entry.key + "' with EXEC_PATH '" + entry.exec_path +
                "' matches no crate source from " +
                display_label(upstream.label),
            )
        if len(matches) > 1:
            fail(
                "rust_env_shard " + producer + ": entry '" +
                entry.key + "' with EXEC_PATH '" + entry.exec_path +
                "' is ambiguous: matches " + str(len(matches)) +
                " crate sources, want exactly one",
            )

    # Unlike the codegen prost adapter, there is no exhaustiveness check:
    # environment identity inputs are selective dimensions, not a closed
    # manifest of every upstream artifact, so unclaimed crate sources are
    # normal. Likewise two keys may share one backing artifact, so no
    # duplicate-claim rejection. The closed-world missing/duplicate/
    # unreported BEP check stays at collection time over the output group.
    bound = []
    for entry in entries:
        if entry.exec_path == "":
            continue
        for f in candidates:
            if _exec_matches(f.path, entry.exec_path):
                bound.append(f)
                break
    out, record = _emit_shard(ctx, producer, ctx.attr.integration, entries)
    return [
        DefaultInfo(files = depset([out])),
        DxEnvPlanInfo(direct = [record], transitive = depset([record])),
        OutputGroupInfo(dx_env_plans = depset([out] + bound)),
    ]

rust_env_shard = rule(
    implementation = _rust_env_shard_impl,
    attrs = {
        "entries": attr.string_list(
            mandatory = True,
            doc = "Explicit identity entries, each KEY|VALUE[|EXEC_PATH]. Exec-bound entries bind one upstream crate source; logical-only entries carry no backing artifact.",
        ),
        "integration": attr.string(
            default = "rust",
            doc = "Language integration class. Only 'rust' is admitted under the slice-1 freeze.",
        ),
        "target": attr.label(
            mandatory = True,
            doc = "One dx_rust_* wrapper target proving the Rust edge via its CrateInfo/TestCrateInfo and crate sources.",
        ),
        "_writer": attr.label(
            default = "//env/env_shard:env_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxEnvShard protobuf.",
        ),
    },
    doc = "Narrow Rust adapter: verifies the dx_rust_* wrapper edge and emits one normalized shard (M25 WP2).",
)

def _env_plan_subject_impl(ctx):
    target = ctx.attr.target
    records = []
    if DxEnvPlanCollectedInfo in target:
        records = target[DxEnvPlanCollectedInfo].records.to_list()
    elif DxEnvPlanInfo in target:
        records = target[DxEnvPlanInfo].transitive.to_list()
    shard_files = []
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if DX_ENV_PLAN_OUTPUT_GROUP in groups:
            shard_files = sorted(
                groups[DX_ENV_PLAN_OUTPUT_GROUP].to_list(),
                key = lambda f: f.basename,
            )
    plan = {
        "files": ",".join([f.basename for f in shard_files]),
        "fingerprint": env_plan_fingerprint(records),
        "label": display_label(ctx.attr.target.label),
        "record_count": str(len(env_plan_merge_records(records))),
    }
    return [
        DefaultInfo(files = depset([])),
        OutputGroupInfo(dx_env_plans = depset(shard_files)),
        DxSubjectInfo(fields = plan),
    ]

env_plan_subject = rule(
    implementation = _env_plan_subject_impl,
    attrs = {
        "target": attr.label(
            aspects = [dx_env_plan_aspect],
            mandatory = True,
            doc = "Fixture target observed with the env plan aspect applied.",
        ),
    },
    doc = "Exposes the merged env plan fingerprint and shard basenames for aspect evidence.",
)
