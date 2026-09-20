"""Repository-root candidates for `dx codegen`, `dx env`, and `dx setup` (M25 WP4, O34).

Contract: `docs/environments/codegen.md`.
"""

REPOSITORY_PATTERN = "//..."

# Every strategy, baseline first. Order mirrors
# `RootStrategy::ALL` in `//cli/roots:dx_roots` and is the deterministic
# tie-break order for benchmark selection: the baseline wins ties.
REPOSITORY_ROOT_STRATEGIES = [
    "recursive-pattern",
    "query-pattern-file",
    "monolithic-aggregate",
    "package-shards",
]

def repository_roots(strategy, monolith = None, shards = [], pattern_file = None):
    """Returns the Bazel command-line patterns for `strategy`."""
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
    doc = "Writes a query-produced repository-root label file (O34 query-pattern-file candidate).",
)

def roots_aggregate(name, deps, **kwargs):
    """One aggregate root: a plain filegroup over `deps`.

    The consuming `bazel build` must still request the plan-collection
    aspects and output groups; this wrapper only reserves the aggregate
    identity so benchmarks can attribute discovery cost.
    """
    native.filegroup(name = name, srcs = deps, **kwargs)
