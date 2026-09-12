"""Typed native-configuration targets for M04 adapters plus M15 Ruff and M17 Biome.

Each initial adapter reads its behavior from exactly one checked-in
tool-owned config file (the native-configuration authority contract):
Buildifier `.buildifier.json`, Taplo `taplo.toml` fragment, Vale `.vale.ini`
plus style/vocab data, rustfmt `rustfmt.toml`, Clippy `clippy.toml`,
Ruff `ruff.toml` (see below for the dedicated-config rule), and Biome
`biome.json` (see below for the config-dir rule).
Source targets opt into a config through `aspect_hints`; the consuming
aspect resolves hints to stages (see `collect_native_configs`). Without a
hint the adapter runs pinned upstream defaults, except Vale, which has no
usable default and fails analysis without a config.

Validation split: `native_config_error` is the pure total predicate over
plain values, exhaustively unit-tested below in spirit (see
`native_config_tests.bzl`). Rule constructors call it and fail analysis on
any error. Non-File members, generated outputs, and wrong extensions fail;
only checked-in source files back a config or its closure.
"""

DxNativeConfigInfo = provider(
    doc = "One tool-owned native config file plus its checked-in closure.",
    fields = {
        "closure": "depset[File]: config plus every data file the tool reaches.",
        "config": "File: the tool-owned config file passed to the adapter.",
        "tool_id": "str: stable built-in tool identifier from REAL_ADAPTERS.",
    },
)

# Tool-owned config filename extensions. The extension is part of the
# transport: it selects the tool's native parser, so a mismatch fails
# analysis instead of silently changing behavior. Ruff recognizes only
# the dedicated `ruff.toml`/`.ruff.toml` basenames (never `pyproject.toml`);
# basename recognition is Gazelle's job, the `.toml` extension check here
# matches the rustfmt/clippy split. Biome takes `biome.json` (or
# `biome.jsonc`): the adapter passes the config's directory as
# `--config-path`, so the directory must hold exactly one config file and
# never linted sources; the `.json` extension check here pins the JSON
# transport while `.jsonc` support stays future work.
_NATIVE_CONFIG_EXTENSIONS = {
    "biome": ".json",
    "buildifier": ".json",
    "clippy": ".toml",
    "ruff": ".toml",
    "rustfmt": ".toml",
    "taplo": ".toml",
    "vale": ".ini",
}

def native_config_extension(tool_id):
    """Returns the required config extension for a real tool ID."""
    if tool_id not in _NATIVE_CONFIG_EXTENSIONS:
        fail("native_config: unknown tool '" + tool_id +
             "': want one of " + ", ".join(sorted(_NATIVE_CONFIG_EXTENSIONS.keys())))
    return _NATIVE_CONFIG_EXTENSIONS[tool_id]

def native_config_error(tool_id, config_path, config_is_source, data):
    """Returns the validation error for a native config, or "" when valid.

    Args:
      tool_id: stable built-in tool identifier from REAL_ADAPTERS.
      config_path: path of the tool-owned config file, "" when absent.
      config_is_source: whether the config is a checked-in source file.
      data: list of structs with `.path` and `.is_source` for closure files.

    Returns:
      The validation error message, or "" when the config is valid.
      Precedence: unknown tool, missing config, generated config, wrong
      extension, generated data member.
    """
    if tool_id not in _NATIVE_CONFIG_EXTENSIONS:
        return ("native_config: unknown tool '" + tool_id + "': want one of " +
                ", ".join(sorted(_NATIVE_CONFIG_EXTENSIONS.keys())))
    if config_path == "":
        return "native_config (" + tool_id + "): src is required"
    if not config_is_source:
        return ("native_config (" + tool_id + "): src must be a checked-in " +
                "source file, got generated " + config_path)
    want = _NATIVE_CONFIG_EXTENSIONS[tool_id]
    if not config_path.endswith(want):
        return ("native_config (" + tool_id + "): src must end in '" + want +
                "', got " + config_path)
    for entry in data:
        if not entry.is_source:
            return ("native_config (" + tool_id + "): data must be " +
                    "checked-in source files, got generated " + entry.path)
    return ""

def collect_native_configs(hints, stage_tools, what):
    """Resolves aspect hints to the configs for a pipeline's stage tools.

    Args:
      hints: list of `DxNativeConfigInfo` (or duck-typed structs with
        `.tool_id` in unit tests).
      stage_tools: ordered tool-ID list for the stages about to run.
      what: label string of the consuming target, for error messages.

    Returns:
      Dict of tool ID to hint for the stage tools that have one. A stage
      tool without a hint is omitted, so the adapter runs pinned upstream
      defaults; duplicate hints for one tool fail analysis as ambiguous.
    """
    by_tool = {}
    for hint in hints:
        if hint.tool_id in by_tool:
            fail("native_config (" + what + "): duplicate aspect_hints " +
                 "for tool '" + hint.tool_id + "'")
        by_tool[hint.tool_id] = hint
    return {tool: by_tool[tool] for tool in stage_tools if tool in by_tool}

def _native_config_impl(ctx):
    tool_id = ctx.attr._tool_id
    config = ctx.file.src
    config_path = config.path if config else ""
    config_is_source = config.is_source if config else False
    data = [struct(path = f.path, is_source = f.is_source) for f in ctx.files.data]
    err = native_config_error(tool_id, config_path, config_is_source, data)
    if err != "":
        fail(err + " (in " + str(ctx.label) + ")")
    closure = depset([config] + ctx.files.data)
    return [
        DefaultInfo(files = closure),
        DxNativeConfigInfo(
            closure = closure,
            config = config,
            tool_id = tool_id,
        ),
    ]

def _make_native_config_rule(tool_id, doc):
    return rule(
        implementation = _native_config_impl,
        attrs = {
            "data": attr.label_list(
                allow_files = True,
                default = [],
                doc = "Checked-in data files the config reaches (Vale styles).",
            ),
            "src": attr.label(
                allow_single_file = True,
                doc = "Checked-in tool-owned config file.",
                mandatory = True,
            ),
            "_tool_id": attr.string(default = tool_id),
        },
        doc = doc,
    )

buildifier_config = _make_native_config_rule(
    "buildifier",
    "Checked-in Buildifier JSON config for Starlark lint/format.",
)

taplo_config = _make_native_config_rule(
    "taplo",
    "Checked-in Taplo TOML config for TOML lint/format.",
)

vale_config = _make_native_config_rule(
    "vale",
    "Checked-in Vale INI config plus styles/vocab data for Markdown lint.",
)

rustfmt_config = _make_native_config_rule(
    "rustfmt",
    "Checked-in rustfmt TOML config for Rust format.",
)

clippy_config = _make_native_config_rule(
    "clippy",
    "Checked-in Clippy TOML config for Rust lint.",
)

ruff_config = _make_native_config_rule(
    "ruff",
    "Checked-in Ruff TOML config (ruff.toml) for Python lint/format. Only the dedicated ruff.toml/.ruff.toml basenames are recognized; pyproject.toml is never a Ruff action input.",
)

biome_config = _make_native_config_rule(
    "biome",
    "Checked-in Biome JSON config (biome.json) for JavaScript/TypeScript/JSON lint/format. The adapter passes the config's directory as --config-path.",
)
