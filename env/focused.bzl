"""Shared focused environment-plan helpers.

Contract: `docs/environments/environment.md`.
"""

load("//quality:sources.bzl", "QualitySourcesInfo")

def focused_direct_sources(target):
    """Returns sorted basenames of direct sources from QualitySourcesInfo."""
    if QualitySourcesInfo not in target:
        return []
    info = target[QualitySourcesInfo]
    out = []
    for class_id in sorted(info.direct_sources.keys()):
        for f in info.direct_sources[class_id].to_list():
            out.append(f.basename)
    return sorted(out)

def focused_simple_plan(direct, label):
    """Builds the simple-language plan dict plus typed source fields."""
    source_count = len(direct)
    has_sources = source_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_sources": str(has_sources),
        "source_count": str(source_count),
        "target": label,
    }
    return struct(
        has_sources = has_sources,
        plan = plan,
        source_count = source_count,
    )

def focused_js_closure(transitive_sources, npm_sources):
    """Collects sorted transitive basenames plus npm closure counts."""
    seen = {}
    for f in transitive_sources:
        seen[f.basename] = True
    return struct(
        has_npm = len(npm_sources) > 0,
        npm_count = len(npm_sources),
        transitive = sorted(seen.keys()),
    )

def focused_js_plan(direct, closure, label):
    """Builds the JS-family plan dict from direct sources plus closure."""
    return {
        "direct_sources": ",".join(direct),
        "has_npm": str(closure.has_npm),
        "npm_source_count": str(closure.npm_count),
        "target": label,
        "transitive_sources": ",".join(closure.transitive),
    }

def focused_typescript_plan(direct, closure, has_tsconfig, label):
    """Builds the TypeScript plan dict extending the JS-family plan."""
    plan = focused_js_plan(direct, closure, label)
    plan["has_tsconfig"] = str(has_tsconfig)
    return plan

def focused_python_transitive(transitive_sources):
    """Returns sorted basenames of first-party `.py` transitive sources.

    Wheel payloads are covered by `wheel_count` plus site-packages import
    roots; raw venv markers carry no import identity and would collapse
    lossily under basename projection."""
    seen = {}
    for f in transitive_sources:
        if f.basename.endswith(".py"):
            seen[f.basename] = True
    return sorted(seen.keys())

def focused_python_plan(direct, transitive, imports, wheel_count, label):
    """Builds the Python plan dict plus the typed wheel flag."""
    has_wheels = wheel_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_wheels": str(has_wheels),
        "imports": ",".join(imports),
        "target": label,
        "transitive_sources": ",".join(transitive),
        "wheel_count": str(wheel_count),
    }
    return struct(
        has_wheels = has_wheels,
        plan = plan,
    )

def _focused_as_list(value):
    """Returns the file list for a list or depset value."""
    if type(value) == "depset":
        return value.to_list()
    return value

def focused_transitive_basenames(files):
    """Returns sorted basenames for a list or depset of files."""
    seen = {}
    for f in _focused_as_list(files):
        seen[f.basename] = True
    return sorted(seen.keys())

def focused_go_transitive(transitive):
    """Returns sorted basenames of `.go` sources in a GoArchive transitive closure."""
    seen = {}
    for archive in _focused_as_list(transitive):
        for f in archive.srcs:
            seen[f.basename] = True
    return sorted(seen.keys())

def focused_dotnet_transitive(refs, transitive_refs):
    """Returns sorted basenames for own refs plus the transitive ref closure."""
    seen = {}
    for f in _focused_as_list(refs):
        seen[f.basename] = True
    for f in _focused_as_list(transitive_refs):
        seen[f.basename] = True
    return sorted(seen.keys())

def focused_test_sources(basenames):
    """Returns sorted test basenames from combined direct plus transitive lists.

    Test shape is `Test` (JVM/Dotnet CamelCase) or `_test` (Go/CC snake_case
    plus Bazel-derived jar/DLL names); `latest` plus `scalatest` stay source-side."""
    seen = {}
    for name in basenames:
        if "Test" in name or "_test" in name:
            seen[name] = True
    return sorted(seen.keys())

def focused_closure_plan(direct, transitive, label):
    """Builds the closure plan dict plus typed source/test fields."""
    tests = focused_test_sources(direct + transitive)
    source_count = len(direct)
    transitive_source_count = len(transitive)
    test_source_count = len(tests)
    has_sources = source_count > 0
    has_tests = test_source_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_sources": str(has_sources),
        "has_tests": str(has_tests),
        "source_count": str(source_count),
        "target": label,
        "test_source_count": str(test_source_count),
        "test_sources": ",".join(tests),
        "transitive_source_count": str(transitive_source_count),
        "transitive_sources": ",".join(transitive),
    }
    return struct(
        has_sources = has_sources,
        has_tests = has_tests,
        plan = plan,
        source_count = source_count,
        test_source_count = test_source_count,
        test_sources = tests,
        transitive_source_count = transitive_source_count,
        transitive_sources = transitive,
    )

def focused_write_plan(ctx, plan):
    """Declares and writes the focused plan JSON output."""
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return out
