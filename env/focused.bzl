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

def focused_write_plan(ctx, plan):
    """Declares and writes the focused plan JSON output."""
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return out
