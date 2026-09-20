"""Normalized codegen plan records (issue #506 WP1, issue #506).

Contract: `docs/environments/codegen.md`, `docs/product/scope.md`.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

DxCodegenPlanInfo = provider(
    doc = "Normalized codegen plan records: direct plus transitive collection.",
    fields = {
        "direct": "List of codegen record structs contributed by this target.",
        "transitive": "Depset of codegen record structs from the closure.",
    },
)

# Aspect-carried merge: Bazel rejects an aspect re-providing its target's
# provider ("provided twice"). See `docs/environments/codegen.md`.
DxCodegenPlanCollectedInfo = provider(
    doc = "Aspect-merged codegen plan records from the traversed closure.",
    fields = {
        "records": "Depset of merged codegen record structs.",
    },
)

# Private output group for collected shards and referenced artifacts.
# Frozen under issue #506; see `docs/environments/codegen.md`.
DX_CODEGEN_PLAN_OUTPUT_GROUP = "dx_codegen_plans"

# Reserved shard suffix for BEP-reported files. See `docs/environments/codegen.md`.
DX_CODEGEN_SHARD_SUFFIX = ".dxcodegen.pb"

# Versioned codegen-pair schema (issue #321). Add pairs via this tuple only.
CODEGEN_SCHEMA_VERSION = 1

# Admitted first-release pairs (slice 1). See `docs/environments/codegen.md`.
DX_CODEGEN_ADMITTED_PAIRS = (
    ("protobuf", "rust"),
)

def codegen_path_error(path):
    """Validates one workspace-relative projection path."""
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
    """Builds one normalized projection entry struct."""
    return struct(
        exec_path = exec_path,
        import_root = import_root,
        logical_path = logical_path,
        namespace = namespace,
        read_only = True,
    )

def codegen_record(producer, language, entries):
    """Builds one normalized contributor record struct."""
    return struct(
        entries = tuple(entries),
        language = language,
        producer = producer,
    )

def codegen_record_error(record):
    """Validates one contributor record."""
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
    another shard)."""
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
    winner is accepted."""
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
    once per selected root."""
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

def codegen_merge_schema_error(records, merged):
    """Validates merged is the normalized form of records (issue #322).

    Checks shape without pinning exact contents, so adding owners or entries
    edits test data only: owners sorted and unique, entries sorted and unique
    per owner, every input entry present deduped, every merged entry sourced,
    and each merged record valid. Exact owner/entry values stay in snapshot
    assertions; this proves normalization."""
    if type(merged) != "list":
        return "codegen merge: want a list, got " + type(merged)
    owners = []
    for record in merged:
        err = codegen_record_error(record)
        if err != "":
            return "codegen merge: " + err
        owners.append((record.producer, record.language))
    if owners != sorted(owners):
        return "codegen merge: owners must sort by (producer, language): " + str(owners)
    seen_owners = {}
    for owner in owners:
        if owner in seen_owners:
            return "codegen merge: duplicate owner " + str(owner)
        seen_owners[owner] = True
    for record in merged:
        keys = [_codegen_entry_key(e) for e in record.entries]
        if keys != sorted(keys):
            return "codegen merge: entries for " + record.producer + " must sort by (logical, root, namespace, exec)"
        seen_keys = {}
        for key in keys:
            if key in seen_keys:
                return "codegen merge: duplicate entry " + str(key) + " for " + record.producer
            seen_keys[key] = True
    input_by_owner = {}
    for record in records:
        owner = (record.producer, record.language)
        owned = input_by_owner.setdefault(owner, {})
        for entry in record.entries:
            owned[_codegen_entry_key(entry)] = True
    merged_by_owner = {}
    for record in merged:
        owner = (record.producer, record.language)
        owned = merged_by_owner.setdefault(owner, {})
        for entry in record.entries:
            owned[_codegen_entry_key(entry)] = True
    if sorted(input_by_owner.keys()) != sorted(merged_by_owner.keys()):
        return "codegen merge: owners must match input owners: " + str(sorted(merged_by_owner.keys())) + " vs " + str(sorted(input_by_owner.keys()))
    for owner in input_by_owner.keys():
        if sorted(input_by_owner[owner].keys()) != sorted(merged_by_owner[owner].keys()):
            return "codegen merge: entries for " + str(owner) + " must match deduped input entries"
    return ""

def codegen_plan_fingerprint(records):
    """Renders the normalized complete-plan hash input."""
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

def codegen_fingerprint_schema_error(fingerprint):
    """Validates a plan fingerprint JSON shape (issue #322).

    Checks structure without pinning exact bytes, so entry additions edit
    test data only: a list of {producer, language, entries} sorted by
    (producer, language) with entries sorted by the full key, each entry
    carrying validated paths plus read-only truth. Exact fingerprint bytes
    stay in snapshot assertions; this proves the hash-input contract."""
    decoded = json.decode(fingerprint)
    if type(decoded) != "list" or len(decoded) == 0:
        return "codegen fingerprint: want a non-empty list"
    owners = []
    for item in decoded:
        if type(item) != "dict":
            return "codegen fingerprint: want objects, got " + type(item)
        for key in ("producer", "language", "entries"):
            if key not in item:
                return "codegen fingerprint: missing key '" + key + "'"
        producer = item["producer"]
        language = item["language"]
        entries = item["entries"]
        if type(producer) != "string" or producer == "":
            return "codegen fingerprint: producer must be non-empty"
        if not (producer.startswith("//") or producer.startswith("@")):
            return "codegen fingerprint: producer '" + producer + "' must be a label"
        if type(language) != "string" or language == "":
            return "codegen fingerprint: language must be non-empty"
        if type(entries) != "list" or len(entries) == 0:
            return "codegen fingerprint: entries must be non-empty for " + producer
        owners.append((producer, language))
        keys = []
        for entry in entries:
            if type(entry) != "dict":
                return "codegen fingerprint: want entry objects for " + producer
            for ekey in ("exec_path", "import_root", "logical_path", "namespace", "read_only"):
                if ekey not in entry:
                    return "codegen fingerprint: missing entry key '" + ekey + "' for " + producer
            if entry["read_only"] != True:
                return "codegen fingerprint: read_only must stay true for " + producer
            err = codegen_path_error(entry["logical_path"])
            if err != "":
                return "codegen fingerprint: " + err
            err = codegen_path_error(entry["import_root"])
            if err != "":
                return "codegen fingerprint: " + err
            err = codegen_exec_error(entry["exec_path"])
            if err != "":
                return "codegen fingerprint: " + err
            keys.append((entry["logical_path"], entry["import_root"], entry["namespace"], entry["exec_path"]))
        if keys != sorted(keys):
            return "codegen fingerprint: entries for " + producer + " must sort by (logical, root, namespace, exec)"
    if owners != sorted(owners):
        return "codegen fingerprint: records must sort by (producer, language): " + str(owners)
    return ""

def codegen_admitted_pairs():
    """Returns the admitted generator/language pairs via registry query.

    Derived from `DX_CODEGEN_ADMITTED_PAIRS`, never duplicated, so adding
    a pair edits the registry data only (issue #321).
    """
    return DX_CODEGEN_ADMITTED_PAIRS

def codegen_schema_error():
    """Validates the versioned codegen-pair schema (issue #321).

    Checks data shape without pinning exact contents, so adding a pair
    edits the admitted data only: version is v1, the list is non-empty
    with unique canonical (schema_kind, language) tuples."""
    if CODEGEN_SCHEMA_VERSION != 1:
        return "codegen: unsupported schema v" + str(CODEGEN_SCHEMA_VERSION) + " (want v1)"
    if type(DX_CODEGEN_ADMITTED_PAIRS) != "tuple" or len(DX_CODEGEN_ADMITTED_PAIRS) == 0:
        return "codegen: want a non-empty admitted-pair tuple (schema v1)"
    seen = {}
    for pair in DX_CODEGEN_ADMITTED_PAIRS:
        if type(pair) != "tuple" or len(pair) != 2:
            return "codegen: bad admitted pair '" + str(pair) + "': want (schema_kind, language)"
        for token in pair:
            if type(token) != "string" or token == "":
                return "codegen: bad admitted pair '" + str(pair) + "': tokens must be non-empty strings"
        if pair in seen:
            return "codegen: duplicate admitted pair '" + str(pair) + "'"
        seen[pair] = True
    return ""

def codegen_pair_error(schema_kind, language):
    """Validates one generator/language pair against the frozen."""
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
    BEP-matching exec-path suffix for the backing artifact."""
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
    """Validates one record and emits its binary shard via the writer."""
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
    `bazel-out`."""
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
            doc = "Generated file class, e.g. 'rust'. Must pair with schema_kind under issue #506.",
        ),
        "schema_kind": attr.string(
            default = "protobuf",
            doc = "Generator schema kind, e.g. 'protobuf'. Only the issue #506 first pair is admitted.",
        ),
        "_writer": attr.label(
            default = "//generation/codegen_shard:codegen_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxCodegenShard protobuf.",
        ),
    },
    doc = "Emits one contributor's normalized binary codegen plan shard (issue #506 WP1).",
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
            doc = "Generated file class. Only 'rust' is admitted with schema_kind 'protobuf' under issue #506.",
        ),
        "proto_rs": attr.label(
            mandatory = True,
            doc = "One rust_prost_library target proving the protobuf->Rust edge via its rust_generated_srcs output group.",
        ),
        "schema_kind": attr.string(
            default = "protobuf",
            doc = "Generator schema kind. Only 'protobuf' is admitted under issue #506.",
        ),
        "_writer": attr.label(
            default = "//generation/codegen_shard:codegen_shard_writer",
            executable = True,
            cfg = "exec",
            doc = "Shard writer emitting the validated binary DxCodegenShard protobuf.",
        ),
    },
    doc = "Narrow protobuf->Rust adapter: verifies the rust_prost_library edge and emits one normalized shard (issue #506 WP1, issue #506).",
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
