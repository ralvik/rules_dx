"""Shared normalized shard pipeline.

Contract: `docs/environments/environment.md`, `docs/environments/codegen.md`.
"""

def plan_shard_exec_matches(file_path, exec_path):
    """Reports whether a Bazel file path satisfies an exec-path suffix."""
    return file_path == exec_path or file_path.endswith("/" + exec_path)

def plan_shard_edge_targets(rule_attr, name):
    """Returns the targets behind one dependency-like rule attribute.

    Args:
      rule_attr: Rule attributes object (e.g. `ctx.rule.attr`).
      name: Attribute name holding the dependency-like value.

    Returns:
      List of Targets; empty when the attribute is absent or None.
    """
    value = getattr(rule_attr, name, [])
    if value == None:
        return []
    if type(value) == "Target":
        return [value]
    return value

def plan_shard_producer_error(producer, kind):
    """Validates one contributor label in observation rendering.

    Args:
      producer: Contributor label string to validate.
      kind: Record kind name used in diagnostics.

    Returns:
      Empty string when valid; otherwise a diagnostic message.
    """
    if producer == "":
        return "invalid " + kind + " record: producer must be a non-empty label"
    if not (producer.startswith("//") or producer.startswith("@")):
        return "invalid " + kind + " record '" + producer + "': producer must be a label in observation rendering"
    return ""

def plan_shard_record_error(producer, kind, second_error, entries, entry_error_of, claim_key_of, duplicate_kind):
    """Validates one contributor record with schema callbacks.

    `second_error` is the schema second-field message without the producer
    prefix (empty when valid). `entry_error_of` validates one entry without
    the producer prefix. `claim_key_of` extracts the within-record uniqueness
    key and `duplicate_kind` names it in diagnostics.

    Args:
      producer: Contributor label string to validate.
      kind: Record kind name used in diagnostics.
      second_error: Schema second-field message without the producer prefix.
      entries: Entry list carried by the record (must be non-empty).
      entry_error_of: Callback validating one entry, returning "" when valid.
      claim_key_of: Callback extracting the within-record uniqueness key.
      duplicate_kind: Noun naming the claim key in duplicate diagnostics.

    Returns:
      Empty string when valid; otherwise a diagnostic message.
    """
    error = plan_shard_producer_error(producer, kind)
    if error != "":
        return error
    if second_error != "":
        return "invalid " + kind + " record '" + producer + "': " + second_error
    if len(entries) == 0:
        return "invalid " + kind + " record '" + producer + "': entries must be non-empty (targets with no contribution carry no record)"
    seen = {}
    for entry in entries:
        error = entry_error_of(entry)
        if error != "":
            return "invalid " + kind + " record '" + producer + "': " + error
        claim = claim_key_of(entry)
        if claim in seen:
            return "invalid " + kind + " record '" + producer + "': duplicate " + duplicate_kind + " '" + claim + "'"
        seen[claim] = True
    return ""

def plan_shard_merge_records(records, owner_of, entry_key_of, make_record):
    """Merges records into deterministic normalized order.

    `owner_of` extracts the (producer, class) owner tuple, `entry_key_of`
    extracts the sortable entry key, and `make_record` rebuilds one record
    from (producer, class, entries). Byte-identical duplicates collapse;
    surviving records sort by owner with entries sorted by key.

    Args:
      records: Source records to merge.
      owner_of: Callback extracting the (producer, class) owner tuple.
      entry_key_of: Callback extracting the sortable entry key.
      make_record: Callback rebuilding one record from (producer, class, entries).

    Returns:
      Merged records sorted by owner with entries sorted by key.
    """
    entries_by_owner = {}
    for record in records:
        owner = owner_of(record)
        owned = entries_by_owner.setdefault(owner, {})
        for entry in record.entries:
            owned[entry_key_of(entry)] = entry
    merged = []
    for owner in sorted(entries_by_owner.keys()):
        entries = [
            entries_by_owner[owner][key]
            for key in sorted(entries_by_owner[owner].keys())
        ]
        merged.append(make_record(owner[0], owner[1], entries))
    return merged

def plan_shard_conflict_error(records, owner_of, entry_key_of, claim_key_of, conflict_prefix, claim_kind):
    """Detects incompatible claims across records with schema callbacks.

    Byte-identical duplicates (same owner plus full entry key) merge
    silently. Any other second claim on one `claim_key_of` fails, listing
    every claimant: no traversal-order winner is accepted.

    Args:
      records: Records whose entries claim uniqueness keys.
      owner_of: Callback extracting the (producer, class) owner tuple.
      entry_key_of: Callback extracting the full entry identity key.
      claim_key_of: Callback extracting the shared uniqueness claim key.
      conflict_prefix: Diagnostic prefix naming the failing context.
      claim_kind: Noun naming the claim key in conflict diagnostics.

    Returns:
      Empty string when no conflict; otherwise a diagnostic message.
    """
    claimants_by_key = {}
    for record in records:
        owner = owner_of(record)
        for entry in record.entries:
            claim = owner + entry_key_of(entry)
            grouped = claimants_by_key.setdefault(claim_key_of(entry), {})
            grouped[claim] = True
    conflicts = []
    for key in sorted(claimants_by_key.keys()):
        claims = sorted(claimants_by_key[key].keys())
        if len(claims) > 1:
            conflicts.append(
                claim_kind + " '" + key + "' claimed by " +
                ", ".join([claim[0] for claim in claims]),
            )
    if not conflicts:
        return ""
    return conflict_prefix + ": " + "; ".join(conflicts)

def plan_shard_fingerprint(records, merge_fn, encode_record):
    """Renders the normalized complete-plan hash input."""
    merged = merge_fn(records)
    return json.encode([encode_record(record) for record in merged])

def plan_shard_aspect_inputs(target, ctx, direct_info, collected_info, output_group, attr_names):
    """Collects direct plus transitive records and files for an aspect.

    Args:
      target: Visited target providing optional direct info and output groups.
      ctx: Aspect context carrying `ctx.rule.attr`.
      direct_info: Provider type exposing the target's direct records.
      collected_info: Provider type exposing dependency collected records.
      output_group: Output group name holding staged result files.
      attr_names: Dependency attribute names to traverse for transitive edges.

    Returns:
      Struct with `direct_files`, `merged_records`, and `transitive_files`.
    """
    direct_records = []
    direct_files = []
    if direct_info in target:
        direct_records = target[direct_info].direct
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if output_group in groups:
            direct_files = groups[output_group].to_list()
    transitive_records = []
    transitive_files = []
    for name in attr_names:
        for dep in plan_shard_edge_targets(ctx.rule.attr, name):
            if collected_info in dep:
                transitive_records.append(dep[collected_info].records)
            if OutputGroupInfo in dep:
                groups = dep[OutputGroupInfo]
                if output_group in groups:
                    transitive_files.append(groups[output_group])
    merged_records = depset(direct_records, transitive = transitive_records)
    return struct(
        direct_files = direct_files,
        merged_records = merged_records,
        transitive_files = transitive_files,
    )

def plan_shard_subject_records(target, direct_info, collected_info):
    """Returns the collected records for a subject, falling back to direct.

    Args:
      target: Subject target carrying optional collected or direct info.
      direct_info: Provider type exposing the target's direct records.
      collected_info: Provider type exposing collected transitive records.

    Returns:
      List of records; empty when neither provider is present.
    """
    if collected_info in target:
        return target[collected_info].records.to_list()
    elif direct_info in target:
        return target[direct_info].transitive.to_list()
    return []

def plan_shard_subject_files(target, output_group):
    """Returns the output-group files for a subject sorted by basename.

    Args:
      target: Subject target carrying optional OutputGroupInfo.
      output_group: Output group name holding staged result files.

    Returns:
      Files sorted by basename; empty when the group is absent.
    """
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if output_group in groups:
            return sorted(
                groups[output_group].to_list(),
                key = lambda f: f.basename,
            )
    return []
