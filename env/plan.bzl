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
"""

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
