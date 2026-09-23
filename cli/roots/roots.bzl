"""Repository-root candidates for `dx codegen`, `dx env`, and `dx setup`.

Contract: `docs/environments/codegen.md`.
"""

REPOSITORY_PATTERN = "//..."

# Every strategy, baseline first. Order mirrors
# `RootStrategy::ALL` in `//cli/roots:dx_roots` and is the deterministic
# tie-break order for fiat selection (ADR 0022): the baseline wins ties.
REPOSITORY_ROOT_STRATEGIES = [
    "recursive-pattern",
    "query-pattern-file",
    "monolithic-aggregate",
    "package-shards",
]

def repository_roots(strategy, monolith = None, shards = [], pattern_file = None):
    """Returns repository-root target patterns for one root strategy.

    Args:
      strategy: Root strategy name from REPOSITORY_ROOT_STRATEGIES.
      monolith: Aggregate root label for monolithic-aggregate, or None.
      shards: Shard patterns for package-shards.
      pattern_file: Pattern file path for query-pattern-file, or None.

    Returns:
      List of repository-root target patterns for the strategy.
    """
    if strategy == "recursive-pattern":
        return [REPOSITORY_PATTERN]
    if strategy == "query-pattern-file":
        if pattern_file == None:
            fail("query-pattern-file roots need `pattern_file`")
        return []
    if strategy == "monolithic-aggregate":
        if monolith == None:
            fail("monolithic-aggregate roots need `monolith`")
        return [monolith]
    if strategy == "package-shards":
        return list(shards)
    fail("unknown repository-root strategy: " + strategy)

def _repository_roots_file_impl(ctx):
    content = "\n".join(ctx.attr.roots)
    if content:
        content += "\n"
    ctx.actions.write(output = ctx.outputs.out, content = content)

repository_roots_file = rule(
    implementation = _repository_roots_file_impl,
    attrs = {
        "roots": attr.string_list(
            mandatory = True,
            doc = "Target patterns written one per line for `--target_pattern_file`.",
        ),
        "out": attr.output(
            mandatory = True,
            doc = "Label file Bazel reads through `--target_pattern_file`.",
        ),
    },
    doc = "Writes a query-produced repository-root label file.",
)

def roots_aggregate(name, deps, **kwargs):
    """One aggregate root: a plain filegroup over `deps`.

    The consuming `bazel build` must still request the plan-collection
    aspects and output groups; this wrapper only reserves the aggregate
    identity so selection can attribute discovery cost.
    """
    native.filegroup(name = name, srcs = deps, **kwargs)
