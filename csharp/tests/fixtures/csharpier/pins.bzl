"""CSharpier check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #486; living
# at head rejected; digests stay owned under issue #797; targets .NET 8.0).
CSHARPIER_VERSION = "1.3.0"

# Distribution identity (exact official tool package as declared DLLs over
# the one managed .NET cohort; no consumer runs `dotnet tool install`;
# digests plus runtime bounds stay owned under issue #797).
CSHARPIER_ARTIFACT = "exact official tool package as declared DLLs (targets .NET 8.0)"
CSHARPIER_RUNTIME = "shared managed .NET runtime cohort; no dotnet tool install"

# Invocation shapes (whole-file rewrite with check/diff mode).
CSHARPIER_CHECK = "csharpier check plus --config-path when hinted (exit 0 clean, exit 1 with unformatted paths when dirty)"
CSHARPIER_FIX = "csharpier format in-place (re-read on exit 0, keep input otherwise)"
CSHARPIER_CONFIG_POLICY = "upstream built-in defaults without config, native interpretation with config"

# Live proof labels (foundation consumers stay green; adapter dispatch
# owned under issue #797).
CSHARP_FIXTURE_HELLO = "//csharp/tests/fixtures/hello:hello_test"

# Rejected: installer on the consumer path, ambient config discovery,
# auto-supplied preset.
CSHARPIER_REJECTED = "dotnet tool install rejected; ambient config discovery rejected; auto-supplied preset rejected"
