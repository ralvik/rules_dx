"""Typed native-configuration targets for adapters plus Ruff, Biome/ESLint, Checkstyle, Scala/.NET, Native, Structured, and file-family cohort.

Contract: `docs/quality/native-configuration.md`.
"""

DxNativeConfigInfo = provider(
    doc = "One tool-owned native config file plus its checked-in closure. " +
          "Forgeable by construction (Starlark providers carry no origin): " +
          "trust comes from the typed constructor's extension/source checks " +
          "plus aspect-time binding, never from the provider alone. " +
          "See: issue #928.",
    fields = {
        "closure": "depset[File]: config plus every data file the tool reaches.",
        "config": "File: the tool-owned config file passed to the adapter.",
        "tool_id": "str: stable built-in tool identifier from REAL_ADAPTERS.",
    },
)

# Tool-owned config filename extensions. The extension is part of the
_NATIVE_CONFIG_EXTENSIONS = {
    "biome": ".json",
    "buf": ".yaml",
    "buildifier": ".json",
    "checkstyle": ".xml",
    "clang_format": ".clang-format",
    "clang_tidy": ".clang-tidy",
    "cppcheck": ".txt",
    "csharpier": ".yaml",
    "djlint": ".toml",
    "eslint": ".js",
    "fsharplint": ".json",
    "qmlformat": ".ini",
    "qmllint": ".ini",
    "ruff": ".toml",
    "rustfmt": ".toml",
    "scalafix": ".conf",
    "scalafmt": ".conf",
    "staticcheck": ".conf",
    "stylelint": ".json",
    "taplo": ".toml",
    "vale": ".ini",
    "yamllint": ".yaml",
}

def native_config_extension(tool_id):
    """Returns the required config extension for a real tool ID."""
    if tool_id not in _NATIVE_CONFIG_EXTENSIONS:
        fail("native_config: unknown tool '" + tool_id +
             "': want one of " + ", ".join(sorted(_NATIVE_CONFIG_EXTENSIONS.keys())))
    return _NATIVE_CONFIG_EXTENSIONS[tool_id]

def native_config_error(tool_id, config_path, config_is_source, data):
    """Returns the validation error for a native config, or "" when valid."""
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
    """Resolves aspect hints to the configs for a pipeline's stage tools."""
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
                doc = "Checked-in data files the config reaches (Vale styles). Unbounded by design: closures span ini/yml and other tool-owned data. See: issue #928.",
            ),
            "src": attr.label(
                allow_single_file = True,
                doc = "Checked-in tool-owned config file.",
                mandatory = True,
            ),
            "_tool_id": attr.string(
                default = tool_id,
                doc = "Stable built-in tool identifier selecting the required config extension.",
            ),
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

ruff_config = _make_native_config_rule(
    "ruff",
    "Checked-in Ruff TOML config (ruff.toml) for Python lint/format. Only the dedicated ruff.toml/.ruff.toml basenames are recognized; pyproject.toml is never a Ruff action input.",
)

biome_config = _make_native_config_rule(
    "biome",
    "Checked-in Biome JSON config (biome.json only, issue #589 wont-fix rejects biome.jsonc; See: docs/quality/native-configuration.md) for JavaScript/TypeScript/JSON lint/format. The adapter passes the config's directory as --config-path.",
)

eslint_config = _make_native_config_rule(
    "eslint",
    "Checked-in ESLint flat config (eslint.config.js) for JavaScript lint. The adapter passes it as -c; no usable upstream default exists.",
)

scalafmt_config = _make_native_config_rule(
    "scalafmt",
    "Checked-in Scalafmt HOCON config (.scalafmt.conf) for Scala format.",
)

scalafix_config = _make_native_config_rule(
    "scalafix",
    "Checked-in Scalafix HOCON config (.scalafix.conf) for Scala lint.",
)

csharpier_config = _make_native_config_rule(
    "csharpier",
    "Checked-in CSharpier YAML config (.csharpierrc) for C# format.",
)

clang_format_config = _make_native_config_rule(
    "clang_format",
    "Checked-in clang-format dotfile (.clang-format) for C/C++/CUDA format. The adapter passes it as --style=file:<path>.",
)

clang_tidy_config = _make_native_config_rule(
    "clang_tidy",
    "Checked-in clang-tidy dotfile (.clang-tidy) for C/C++ lint. The adapter passes it as --config-file.",
)

cppcheck_config = _make_native_config_rule(
    "cppcheck",
    "Checked-in cppcheck suppressions list (.txt) for C/C++ lint. The adapter passes it as --suppressions-list.",
)

staticcheck_config = _make_native_config_rule(
    "staticcheck",
    "Checked-in staticcheck HOCON config (staticcheck.conf) for Go lint.",
)

fsharplint_config = _make_native_config_rule(
    "fsharplint",
    "Checked-in FSharpLint JSON config (fsharplint.json) for F# lint.",
)

checkstyle_config = _make_native_config_rule(
    "checkstyle",
    "Checked-in Checkstyle XML config for Java lint. The adapter passes it as -c; no usable upstream default exists.",
)

buf_config = _make_native_config_rule(
    "buf",
    "Checked-in Buf YAML config (buf.yaml) for Protobuf format/lint.",
)

qmlformat_config = _make_native_config_rule(
    "qmlformat",
    "Checked-in qmlformat INI config (.qmlformat.ini) for QML format.",
)

qmllint_config = _make_native_config_rule(
    "qmllint",
    "Checked-in qmllint INI config (.qmllint.ini) for QML lint.",
)

stylelint_config = _make_native_config_rule(
    "stylelint",
    "Checked-in Stylelint JSON config for CSS lint. The adapter passes it as -c; upstream defaults apply unhinted.",
)

djlint_config = _make_native_config_rule(
    "djlint",
    "Checked-in djlint TOML config for HTML-template lint/format.",
)

yamllint_config = _make_native_config_rule(
    "yamllint",
    "Checked-in yamllint YAML config for YAML lint.",
)
