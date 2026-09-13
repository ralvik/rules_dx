"""Normalized codegen plan records (M25 WP1, O33).

`DxCodegenPlanInfo` is the in-memory shape behind the binary
`DxCodegenShard` wire schema (`codegen.proto`): each contributing
configured target carries its direct projection records while collectors
forward the transitive merge. Shard emission, the collecting aspect, and
the private `dx_codegen_plans` output group land in a later WP1 slice;
this file freezes the private names, the admitted first pair, and the
pure validation/merge semantics those pieces share.

O33 freeze (slice 1): the only first-release generator/language pair is
Protocol Buffer schema to Rust through `rust_prost_library` (dogfooded by
`//quality:result_proto_rs` and `//generation:result_proto_rs`). GraphQL
and any Python/Node projection have no support-matrix cell and stay
deferred to later M25 slices under O33; they are not dropped and no
adapter claims them here.

Conflict rule (from `docs/environments/codegen.md`): multiple producers
claiming an incompatible language import path fail before selection; no
traversal-order winner is accepted. Byte-identical duplicate records (the
same producer, language, path, root, and namespace) merge silently, since
transitive collection reaches one record through many routes.

Contract: `docs/environments/codegen.md` (provider contract, filesystem
projection), `docs/product/scope.md` (automatic workflows).
"""

DxCodegenPlanInfo = provider(
    doc = "Normalized codegen plan records: direct plus transitive collection.",
    fields = {
        "direct": "List of codegen record structs contributed by this target.",
        "transitive": "Depset of codegen record structs from the closure.",
    },
)

# Private output group carrying collected shards plus every generated
# artifact referenced by them. Frozen under O33; the aspect requests it.
DX_CODEGEN_PLAN_OUTPUT_GROUP = "dx_codegen_plans"

# Reserved controlled filename suffix recognizing shards among
# BEP-reported files. Frozen under O33; the CLI rejects unreported,
# missing, or duplicate artifacts and never scans `bazel-out`.
DX_CODEGEN_SHARD_SUFFIX = ".dxcodegen.pb"

# Admitted first-release generator/language pairs, slice 1. Each entry is
# a (schema kind, generated file class) tuple. Later M25 slices extend
# this tuple only through the O33 qualification recorded above.
DX_CODEGEN_ADMITTED_PAIRS = (
    ("protobuf", "rust"),
)

def codegen_path_error(path):
    """Validates one workspace-relative projection path.

    Args:
      path: candidate logical path or import root.

    Returns:
      "" when valid, else the failure reason: empty, absolute, a `.` or
      `..` segment, or a backslash separator.
    """
    if path == "":
        return "invalid codegen path '': must be a non-empty workspace-relative path"
    if path.startswith("/"):
        return "invalid codegen path '" + path + "': must not be absolute"
    if "\\" in path:
        return "invalid codegen path '" + path + "': must not contain '\\'"
    for segment in path.split("/"):
        if segment == "." or segment == "..":
            return "invalid codegen path '" + path + "': must not contain '.' or '..' segments"
    return ""

def codegen_entry(logical_path, import_root, namespace = ""):
    """Builds one normalized projection entry struct.

    Args:
      logical_path: deterministic workspace-relative generated path.
      import_root: language import/source root, workspace-relative.
      namespace: required package/namespace semantics, or "".

    Returns:
      A struct with `logical_path`, `import_root`, `namespace`, and
      `read_only` (always True: projections are read-only context).
    """
    return struct(
        import_root = import_root,
        logical_path = logical_path,
        namespace = namespace,
        read_only = True,
    )

def codegen_record(producer, language, entries):
    """Builds one normalized contributor record struct.

    Args:
      producer: contributing target label in observation rendering.
      language: generated file class, e.g. "rust".
      entries: non-empty list of `codegen_entry` structs.

    Returns:
      A struct with `producer`, `language`, and `entries` (as a tuple).
    """
    return struct(
        entries = tuple(entries),
        language = language,
        producer = producer,
    )

def codegen_record_error(record):
    """Validates one contributor record.

    Args:
      record: candidate `codegen_record` struct.

    Returns:
      "" when valid, else the failure reason naming the bad producer,
      language, entry path, or within-record duplicate logical path.
    """
    if record.producer == "":
        return "invalid codegen record: producer must be a non-empty label"
    if not (record.producer.startswith("//") or record.producer.startswith("@")):
        return "invalid codegen record '" + record.producer + "': producer must be a label in observation rendering"
    if record.language == "":
        return "invalid codegen record '" + record.producer + "': language must be a non-empty file class"
    if len(record.entries) == 0:
        return "invalid codegen record '" + record.producer + "': entries must be non-empty (targets with no contribution carry no record)"
    seen = {}
    for entry in record.entries:
        error = codegen_path_error(entry.logical_path)
        if error != "":
            return "invalid codegen record '" + record.producer + "': " + error
        error = codegen_path_error(entry.import_root)
        if error != "":
            return "invalid codegen record '" + record.producer + "': " + error
        if entry.logical_path in seen:
            return "invalid codegen record '" + record.producer + "': duplicate logical path '" + entry.logical_path + "'"
        seen[entry.logical_path] = True
    return ""

def _codegen_entry_key(entry):
    return (entry.logical_path, entry.import_root, entry.namespace)

def codegen_conflict_error(records):
    """Detects incompatible logical-path claims across records.

    Byte-identical duplicates (same producer, language, path, root, and
    namespace) merge silently. Any other second claim on one logical path
    fails, listing every claimant: no traversal-order winner is accepted.

    Args:
      records: list of `codegen_record` structs.

    Returns:
      "" when conflict-free, else the failure reason listing every
      collided logical path with its sorted claimant producers.
    """
    claimants_by_path = {}
    for record in records:
        for entry in record.entries:
            key = (record.producer, record.language) + _codegen_entry_key(entry)
            paths = claimants_by_path.setdefault(entry.logical_path, {})
            paths[key] = True
    conflicts = []
    for path in sorted(claimants_by_path.keys()):
        claims = sorted(claimants_by_path[path].keys())
        if len(claims) > 1:
            conflicts.append(
                "logical path '" + path + "' claimed by " +
                ", ".join([claim[0] for claim in claims]),
            )
    if not conflicts:
        return ""
    return "codegen path conflict: " + "; ".join(conflicts)

def codegen_merge_records(records):
    """Merges records into deterministic normalized order.

    Byte-identical duplicate entries collapse; surviving records sort
    by (producer, language) with entries sorted by logical path. The
    rendering is the normalized complete-plan form the CLI hashes;
    repository roots emit no second closure manifest, so shared closures
    serialize once per record, not once per selected root.

    Args:
      records: list of `codegen_record` structs.

    Returns:
      The deduplicated record list in normalized order.
    """
    entries_by_owner = {}
    for record in records:
        owner = (record.producer, record.language)
        owned = entries_by_owner.setdefault(owner, {})
        for entry in record.entries:
            owned[_codegen_entry_key(entry)] = entry
    merged = []
    for owner in sorted(entries_by_owner.keys()):
        entries = [
            entries_by_owner[owner][key]
            for key in sorted(entries_by_owner[owner].keys())
        ]
        merged.append(codegen_record(owner[0], owner[1], entries))
    return merged

def codegen_plan_fingerprint(records):
    """Renders the normalized complete-plan hash input.

    Args:
      records: list of `codegen_record` structs.

    Returns:
      Deterministic JSON over the merged records: one object per record
      with producer, language, and entries sorted by logical path.
    """
    merged = codegen_merge_records(records)
    return json.encode([
        {
            "entries": [
                {
                    "import_root": entry.import_root,
                    "logical_path": entry.logical_path,
                    "namespace": entry.namespace,
                    "read_only": entry.read_only,
                }
                for entry in record.entries
            ],
            "language": record.language,
            "producer": record.producer,
        }
        for record in merged
    ])
