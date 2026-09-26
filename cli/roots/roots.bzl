
REPOSITORY_PATTERN = "//..."

REPOSITORY_ROOT_STRATEGIES = [
    "recursive-pattern",
    "query-pattern-file",
    "monolithic-aggregate",
    "package-shards",
]

def repository_roots(strategy, monolith = None, shards = [], pattern_file = None):
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
        ),
        "out": attr.output(
            mandatory = True,
        ),
    },
)

def roots_aggregate(name, deps, **kwargs):
    native.filegroup(name = name, srcs = deps, **kwargs)
