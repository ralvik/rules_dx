"""Fantomas check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference line (7.x stable; 8.0.0 alphas rejected; digests stay
FANTOMAS_LINE = "7.x stable"

# Distribution identity (exact official tool package as declared DLLs over
# the one managed .NET cohort; no consumer runs `dotnet tool install`;
FANTOMAS_ARTIFACT = "exact official tool package as declared DLLs over the managed .NET cohort"
FANTOMAS_RUNTIME = "shared managed .NET runtime cohort; no dotnet tool install"

# Invocation shapes (whole-file rewrite with check/diff mode).
FANTOMAS_CHECK = "fantomas check --json (exit 0 all unchanged, exit 99 with needs-formatting, exit 1 operational failure)"
FANTOMAS_FIX = "fantomas in-place format (re-read on exit 0, keep input otherwise)"
FANTOMAS_CONFIG_POLICY = "upstream built-in defaults without config, native .editorconfig interpretation with config"

# Live proof labels (foundation consumers stay green; adapter dispatch
FSHARP_FIXTURE_HELLO = "//fsharp/tests/fixtures/hello:hello_lib"

# Rejected: installer on the consumer path, non-JSON console-parse,
# auto-supplied preset.
FANTOMAS_REJECTED = "dotnet tool install rejected; non-JSON console-parse rejected; auto-supplied preset rejected"
