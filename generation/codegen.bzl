"""Normalized codegen plan records (M25 WP1, O33).

`DxCodegenPlanInfo` is the in-memory shape behind the binary
`DxCodegenShard` wire schema (`codegen.proto`): each shard-emitting rule
carries its direct projection record. `DxCodegenPlanCollectedInfo` is the
separate aspect-carried merge: Bazel rejects an aspect that re-provides its
target's own provider ("provided twice"), so the collecting aspect never
returns `DxCodegenPlanInfo` and contributing rules never return the
collected provider. Shard emission, the collecting aspect, and the private
`dx_codegen_plans` output group land in WP1 slice 2 (this file); the
narrow prost adapter over the frozen slice-1 semantics is here as well.

O33 freeze (slice 1): the only first-release generator/language pair is
Protocol Buffer schema to Rust through `rust_prost_library` (dogfooded by
`//quality:result_proto_rs` and `//generation:result_proto_rs`). GraphQL
and any Python/Node projection have no support-matrix cell and stay
deferred to later M25 slices under O33; they are not dropped and no
adapter claims them here.

Conflict rule (from `docs/environments/codegen.md`): multiple producers
claiming an incompatible language import path fail before selection; no
traversal-order winner is accepted. Byte-identical duplicate records (the
same producer, language, path, root, namespace, and exec path) merge
silently, since transitive collection reaches one record through many
routes.

Slice 4 adds the optional per-entry `exec_path` BEP-matching suffix
(field 5 on the wire): empty means a logical-only entry requiring no
materialized artifact; non-empty must resolve to exactly one BEP-reported
non-shard artifact (suffix match so output bases differ) and every
non-shard BEP artifact must be claimed, otherwise collection fails with
missing/duplicate/unreported before projection planning. The fingerprint
covers the exec path, so the plan identity binds logical mappings to
their backing artifacts.

Contract: `docs/environments/codegen.md` (provider contract, filesystem
projection), `docs/product/scope.md` (automatic workflows).

Slice 2 (this file) adds the shard-emission rule, the collecting
aspect, and the narrow prost adapter over the frozen slice-1 semantics.
No new pair, name, or merge semantic lands here.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

DxCodegenPlanInfo = provider(
    doc = "Normalized codegen plan records: direct plus transitive collection.",
    fields = {
        "direct": "List of codegen record structs contributed by this target.",
        "transitive": "Depset of codegen record structs from the closure.",
    },
)

# Aspect-carried merge over DxCodegenPlanInfo contributors. Kept distinct
# because Bazel reports "provided twice" when an aspect returns the same
# provider its target already provides: shard rules carry direct records
# in DxCodegenPlanInfo, the aspect merges them here, and fixtures prefer
# this provider with a direct-rule fallback.
DxCodegenPlanCollectedInfo = provider(
    doc = "Aspect-merged codegen plan records from the traversed closure.",
    fields = {
        "records": "Depset of merged codegen record structs.",
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

def codegen_entry(logical_path, import_root, namespace = "", exec_path = ""):
    """Builds one normalized projection entry struct.

    Args:
      logical_path: deterministic workspace-relative generated path.
      import_root: language import/source root, workspace-relative.
      namespace: required package/namespace semantics, or "".
      exec_path: BEP-matching suffix for the backing artifact, or "" for
        a logical-only entry requiring no materialized artifact.

    Returns:
      A struct with `logical_path`, `import_root`, `namespace`,
      `exec_path`, and `read_only` (always True: projections are
      read-only context).
    """
    return struct(
        exec_path = exec_path,
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
        error = codegen_exec_error(entry.exec_path)
        if error != "":
            return "invalid codegen record '" + record.producer + "': " + error
        if entry.logical_path in seen:
            return "invalid codegen record '" + record.producer + "': duplicate logical path '" + entry.logical_path + "'"
        seen[entry.logical_path] = True
    return ""

def codegen_exec_error(path):
    """Validates one BEP-matching exec-path suffix.

    Empty means a logical-only entry requiring no artifact. Non-empty
    follows the same workspace-relative shape rules as logical paths
    and never uses the reserved shard suffix (a shard never backs
    another shard).

    Args:
      path: candidate exec-path suffix.

    Returns:
      "" when valid, else the failure reason.
    """
    if path == "":
        return ""
    if path.endswith(DX_CODEGEN_SHARD_SUFFIX):
        return "invalid codegen exec path '" + path + "': must not use the reserved shard suffix '" + DX_CODEGEN_SHARD_SUFFIX + "'"
    error = codegen_path_error(path)
    if error != "":
        return error.replace("invalid codegen path", "invalid codegen exec path", 1)
    return ""

def _codegen_entry_key(entry):
    return (entry.logical_path, entry.import_root, entry.namespace, entry.exec_path)

def codegen_conflict_error(records):
    """Detects incompatible logical-path claims across records.

    Byte-identical duplicates (same producer, language, path, root,
    namespace, and exec path) merge silently. Any other second claim on
    one logical path fails, listing every claimant: no traversal-order
    winner is accepted.

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
    by (producer, language) with entries sorted by (logical path,
    import root, namespace, exec path). The rendering is the normalized
    complete-plan form the CLI hashes; repository roots emit no second
    closure manifest, so shared closures serialize once per record, not
    once per selected root.

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
      with producer, language, and entries sorted by (logical path,
      import root, namespace, exec path), each entry carrying its
      exec-path suffix so the identity binds mappings to artifacts.
    """
    merged = codegen_merge_records(records)
    return json.encode([
        {
            "entries": [
                {
                    "exec_path": entry.exec_path,
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

def codegen_pair_error(schema_kind, language):
    """Validates one generator/language pair against the O33 freeze.

    Args:
      schema_kind: schema kind, e.g. "protobuf".
      language: generated file class, e.g. "rust".

    Returns:
      "" when the pair is admitted, else the failure reason.
    """
    if (schema_kind, language) in DX_CODEGEN_ADMITTED_PAIRS:
        return ""
    return (
        "unsupported codegen pair ('" + schema_kind + "', '" + language + "'): " +
        "admitted first-release pairs are " + str(DX_CODEGEN_ADMITTED_PAIRS)
    )

def _parse_entry_spec(spec, label_text):
    """Parses one LOGICAL|ROOT|NAMESPACE[|EXEC] entry spec.

    The three-part form declares a logical-only entry (empty exec path,
    requiring no materialized artifact). The four-part form declares the
    BEP-matching exec-path suffix for the backing artifact.

    Args:
      spec: raw entry string with two or three "|" separators.
      label_text: owning label rendering for diagnostics.

    Returns:
      A `codegen_entry` struct.
    """
    parts = spec.split("|")
    if len(parts) == 3:
        return codegen_entry(parts[0], parts[1], parts[2])
    if len(parts) == 4:
        return codegen_entry(parts[0], parts[1], parts[2], parts[3])
    fail(
        "dx_codegen_shard " + label_text +
        ": bad entry " + repr(spec) +
        ": want LOGICAL_PATH|IMPORT_ROOT|NAMESPACE[|EXEC_PATH]",
    )

def _emit_shard(ctx, producer, language, entry_structs):
    """Validates one record and emits its binary shard via the writer.

    Args:
      ctx: rule context with executable `_writer`.
      producer: contributor label in observation rendering.
      language: generated file class.
      entry_structs: list of `codegen_entry` structs.

    Returns:
      The declared shard file.
    """
    record = codegen_record(producer, language, entry_structs)
    record_error = codegen_record_error(record)
    if record_error != "":
        fail("dx_codegen_shard " + producer + ": " + record_error)
    out = ctx.actions.declare_file(ctx.label.name + DX_CODEGEN_SHARD_SUFFIX)
    args = ctx.actions.args()
    args.add("--producer", producer)
    args.add("--language", language)
    for entry in entry_structs:
        if entry.exec_path == "":
            args.add(
                "--entry",
                entry.logical_path + "|" + entry.import_root + "|" + entry.namespace,
            )
        else:
            args.add(
                "--entry",
                entry.logical_path + "|" + entry.import_root + "|" + entry.namespace + "|" + entry.exec_path,
            )
    args.add("--output", out.path)
    ctx.actions.run(
        executable = ctx.executable._writer,
        arguments = [args],
        outputs = [out],
        mnemonic = "DxCodegenShard",
        progress_message = "Dx codegen shard %{label}",
    )
    return out, record

def _exec_matches(file_path, exec_path):
    """Reports whether a Bazel file path satisfies an exec-path suffix.

    Suffix matching (on "/" boundaries, plus exact equality) lets one
    logical entry resolve under different output bases without scanning
    `bazel-out`.

    Args:
      file_path: Bazel `File.path` of a generated artifact.
      exec_path: non-empty BEP-matching suffix from an entry.

    Returns:
      True when `file_path` equals `exec_path` or ends with
      `"/" + exec_path`.
    """
    return file_path == exec_path or file_path.endswith("/" + exec_path)

def _dx_codegen_shard_impl(ctx):
    producer = display_label(ctx.label)
    pair_error = codegen_pair_error(ctx.attr.schema_kind, ctx.attr.language)
    if pair_error != "":
        fail("dx_codegen_shard " + producer + ": " + pair_error)
    entries = [_parse_entry_spec(spec, producer) for spec in ctx.attr.entries]
    out, record = _emit_shard(ctx, producer, ctx.attr.language, entries)
    return [
        DefaultInfo(files = depset([out])),
        DxCodegenPlanInfo(direct = [record], transitive = depset([record])),
        OutputGroupInfo(dx_codegen_plans = depset([out])),
    ]

dx_codegen_shard = rule(
    implementation = _dx_codegen_shard_impl,
    attrs = {
        "deps": attr.label_list(
            default = [],
            doc = "Graph edges the collecting aspect traverses; contributes no records itself.",
        ),
        "entries": attr.string_list(
            mandatory = True,
            doc = "Non-empty projection entries, each LOGICAL_PATH|IMPORT_ROOT|NAMESPACE[|EXEC_PATH].",
        ),
        "language": attr.string(
            mandatory = True,
            doc = "Generated file class, e.g. 'rust'. Must pair with schema_kind under O33.",
        ),
        "schema_kind": attr.string(
            default = "protobuf",
            doc = "Generator schema kind, e.g. 'protobuf'. Only the O33 first pair is admitted.",
        ),
        "_writer": attr.label(
            default = "//generation/codegen_shard:codegen_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxCodegenShard protobuf.",
        ),
    },
    doc = "Emits one contributor's normalized binary codegen plan shard (M25 WP1).",
)

def _edge_targets(rule_attr, name):
    value = getattr(rule_attr, name, [])
    if value == None:
        return []
    if type(value) == "Target":
        return [value]
    return value

# Narrow traversal edges for the collecting aspect: the shard rule's own
# `deps`, the prost library's `proto` edge, and the prost adapter's
# `proto_rs` edge. No `data`, `srcs`, `DefaultInfo`, or broad unions are
# traversed: collectors consume only normalized providers.
_CODEGEN_ASPECT_ATTRS = ["deps", "proto", "proto_rs"]

def _dx_codegen_plan_aspect_impl(target, ctx):
    direct_records = []
    direct_files = []
    if DxCodegenPlanInfo in target:
        direct_records = target[DxCodegenPlanInfo].direct
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if DX_CODEGEN_PLAN_OUTPUT_GROUP in groups:
            direct_files = groups[DX_CODEGEN_PLAN_OUTPUT_GROUP].to_list()
    transitive_records = []
    transitive_files = []
    for name in _CODEGEN_ASPECT_ATTRS:
        for dep in _edge_targets(ctx.rule.attr, name):
            if DxCodegenPlanCollectedInfo in dep:
                transitive_records.append(dep[DxCodegenPlanCollectedInfo].records)
            if OutputGroupInfo in dep:
                groups = dep[OutputGroupInfo]
                if DX_CODEGEN_PLAN_OUTPUT_GROUP in groups:
                    transitive_files.append(groups[DX_CODEGEN_PLAN_OUTPUT_GROUP])
    merged_records = depset(direct_records, transitive = transitive_records)
    conflict = codegen_conflict_error(merged_records.to_list())
    if conflict != "":
        fail("dx_codegen_plan_aspect on " + display_label(target.label) + ": " + conflict)
    return [
        DxCodegenPlanCollectedInfo(records = merged_records),
        OutputGroupInfo(dx_codegen_plans = depset(direct_files, transitive = transitive_files)),
    ]

dx_codegen_plan_aspect = aspect(
    implementation = _dx_codegen_plan_aspect_impl,
    attr_aspects = _CODEGEN_ASPECT_ATTRS,
    doc = "Collects normalized codegen plan records and shard files along narrow codegen edges.",
)

def _prost_codegen_shard_impl(ctx):
    producer = display_label(ctx.label)
    pair_error = codegen_pair_error(ctx.attr.schema_kind, ctx.attr.language)
    if pair_error != "":
        fail("prost_codegen_shard " + producer + ": " + pair_error)
    upstream = ctx.attr.proto_rs
    if OutputGroupInfo not in upstream:
        fail(
            "prost_codegen_shard " + producer + ": upstream " +
            display_label(upstream.label) + " carries no OutputGroupInfo",
        )
    groups = upstream[OutputGroupInfo]
    if "rust_generated_srcs" not in groups:
        fail(
            "prost_codegen_shard " + producer + ": upstream " +
            display_label(upstream.label) +
            " has no rust_generated_srcs output group (not a rust_prost_library)",
        )
    generated = groups["rust_generated_srcs"].to_list()
    if len(generated) == 0:
        fail(
            "prost_codegen_shard " + producer + ": upstream " +
            display_label(upstream.label) + " emitted no generated Rust sources",
        )
    entries = [_parse_entry_spec(spec, producer) for spec in ctx.attr.entries]
    for entry in entries:
        if entry.exec_path == "":
            fail(
                "prost_codegen_shard " + producer + ": entry '" +
                entry.logical_path + "' needs an EXEC_PATH suffix binding it to one rust_generated_srcs artifact (logical-only entries carry no backing artifact)",
            )
    for entry in entries:
        matches = [f for f in generated if _exec_matches(f.path, entry.exec_path)]
        if len(matches) == 0:
            fail(
                "prost_codegen_shard " + producer + ": entry '" +
                entry.logical_path + "' with EXEC_PATH '" + entry.exec_path +
                "' matches no rust_generated_srcs artifact from " +
                display_label(upstream.label),
            )
        if len(matches) > 1:
            fail(
                "prost_codegen_shard " + producer + ": entry '" +
                entry.logical_path + "' with EXEC_PATH '" + entry.exec_path +
                "' is ambiguous: matches " + str(len(matches)) +
                " rust_generated_srcs artifacts, want exactly one",
            )
    for f in generated:
        claimants = [entry for entry in entries if _exec_matches(f.path, entry.exec_path)]
        if len(claimants) == 0:
            fail(
                "prost_codegen_shard " + producer + ": unreported rust_generated_srcs artifact '" +
                f.path + "' from " + display_label(upstream.label) +
                ": every generated artifact needs one claiming entry",
            )
        if len(claimants) > 1:
            fail(
                "prost_codegen_shard " + producer + ": duplicate claim on rust_generated_srcs artifact '" +
                f.path + "': claimed by " + str(len(claimants)) + " entries, want exactly one",
            )
    out, record = _emit_shard(ctx, producer, ctx.attr.language, entries)
    return [
        DefaultInfo(files = depset([out])),
        DxCodegenPlanInfo(direct = [record], transitive = depset([record])),
        OutputGroupInfo(dx_codegen_plans = depset([out] + generated)),
    ]

prost_codegen_shard = rule(
    implementation = _prost_codegen_shard_impl,
    attrs = {
        "entries": attr.string_list(
            mandatory = True,
            doc = "Explicit logical projection entries, each LOGICAL_PATH|IMPORT_ROOT|NAMESPACE|EXEC_PATH. Paths are never inferred from the upstream action; every entry binds one rust_generated_srcs artifact and every generated artifact needs one claimant.",
        ),
        "language": attr.string(
            default = "rust",
            doc = "Generated file class. Only 'rust' is admitted with schema_kind 'protobuf' under O33.",
        ),
        "proto_rs": attr.label(
            mandatory = True,
            doc = "One rust_prost_library target proving the protobuf->Rust edge via its rust_generated_srcs output group.",
        ),
        "schema_kind": attr.string(
            default = "protobuf",
            doc = "Generator schema kind. Only 'protobuf' is admitted under O33.",
        ),
        "_writer": attr.label(
            default = "//generation/codegen_shard:codegen_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxCodegenShard protobuf.",
        ),
    },
    doc = "Narrow protobuf->Rust adapter: verifies the rust_prost_library edge and emits one normalized shard (M25 WP1, O33).",
)

def _codegen_plan_subject_impl(ctx):
    target = ctx.attr.target
    records = []
    if DxCodegenPlanCollectedInfo in target:
        records = target[DxCodegenPlanCollectedInfo].records.to_list()
    elif DxCodegenPlanInfo in target:
        records = target[DxCodegenPlanInfo].transitive.to_list()
    shard_files = []
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if DX_CODEGEN_PLAN_OUTPUT_GROUP in groups:
            shard_files = sorted(
                groups[DX_CODEGEN_PLAN_OUTPUT_GROUP].to_list(),
                key = lambda f: f.basename,
            )
    plan = {
        "files": ",".join([f.basename for f in shard_files]),
        "fingerprint": codegen_plan_fingerprint(records),
        "label": display_label(ctx.attr.target.label),
        "record_count": str(len(codegen_merge_records(records))),
    }
    return [
        DefaultInfo(files = depset([])),
        OutputGroupInfo(dx_codegen_plans = depset(shard_files)),
        DxSubjectInfo(fields = plan),
    ]

codegen_plan_subject = rule(
    implementation = _codegen_plan_subject_impl,
    attrs = {
        "target": attr.label(
            aspects = [dx_codegen_plan_aspect],
            mandatory = True,
            doc = "Fixture target observed with the codegen plan aspect applied.",
        ),
    },
    doc = "Exposes the merged codegen plan fingerprint and shard basenames for aspect evidence.",
)
