"""Scala + .NET quality defaults pins (issue #486).

Contract: `docs/product/support-matrix.md#provisional-default-quality-tools`,
`docs/quality/native-configuration.md#authority`.

Qualifies the provisional Scala + .NET format plus lint defaults against the
native-configuration contract with no hidden presets. Versions below are the
pinned upstream releases from the initial artifact research; rule-sets are
upstream built-in defaults unless an applicable checked-in native config
supplies policy. Adapters add only transport/hermetic settings. Exact
artifact digests plus adapter mappings stay owned under issue #417.
"""

# Pinned upstream versions (initial artifact research observations, now
# qualified seed-only under #486; living at head plus alphas rejected).
SCALAFMT_VERSION = "3.11.4"
SCALAFIX_VERSION = "0.14.7"
CSHARPIER_VERSION = "1.3.0"
FANTOMAS_LINE = "7.x stable"
FSHARPLINT_VERSION = "0.27.0"

# Upstream distribution identities (managed-JVM route for Scalafmt/Scalafix
# over the shared managed JDK plus the Scala Maven-lock story; exact-package
# plus shared-.NET-runtime route for CSharpier/Fantomas/FSharpLint as
# declared DLLs over one managed .NET cohort; Roslyn is SDK-coupled with no
# separate artifact; digests stay owned under issue #417, never reconstructed
# from modules and never installed via `dotnet tool install`).
SCALAFMT_ARTIFACT = "compatible JVM artifact over the shared managed JDK"
SCALAFIX_ARTIFACT = "semantic-rule artifacts over the shared managed JDK"
CSHARPIER_ARTIFACT = "exact official tool package as declared DLLs (targets .NET 8.0)"
FANTOMAS_ARTIFACT = "exact official tool package as declared DLLs over the managed .NET cohort"
FSHARPLINT_ARTIFACT = "exact official tool package over the managed .NET cohort (targets .NET 8.0)"
ROSLYN_COUPLING = "SDK-built-in CA analyzers following the qualified .NET SDK, no separate acquisition"

# Rejected lines (never pinned here).
REJECTED_SCALAFMT_HEAD = "newer than 3.11.4 observed, not pinned; living at head rejected"
REJECTED_FANTOMAS_ALPHA = "8.0.0 alphas target the next FSharp.Core/.NET and are not stable; rejected"

# Native-configuration sole policy (no hidden presets). Without an applicable
# checked-in native config the pinned tool uses its upstream built-in
# behavioral defaults; with a config the tool interprets it natively.
NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

# Qualified rule-set resolutions (upstream built-in defaults, not rules_dx presets).
SCALAFIX_POLICY = "no auto-supplied OrganizeImports plus RemoveUnused preset; recommended built-ins plus OrganizeImports plus RemoveUnused require checked-in .scalafix.conf, upstream built-in defaults without config, native interpretation with config"
ROSLYN_POLICY = "SDK default analysis mode is the upstream built-in default mode, not a rules_dx preset"
STYLECOP_POLICY = "StyleCop stays opt-in, never the default; its style rules can contradict the built-in IDE rules"
FSHARPLINT_POLICY = "default ruleset with formatting rules off is the upstream built-in default ruleset minus formatting; Fantomas owns formatting"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: auto preset, --enable=all-style opt-in maxima"
AUTO_PRESET_REJECTED = "auto preset rejected: provisional inputs only, never an automatically supplied config"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet under issue #417).
SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"
CSHARP_FIXTURE_HELLO = "//csharp/tests/fixtures/hello:hello_test"
FSHARP_FIXTURE_HELLO = "//fsharp/tests/fixtures/hello:hello_lib"

# Rejected: hidden presets plus unpinned versions.
SCALA_DOTNET_REJECTED = "hidden presets rejected: no auto-supplied Scalafix preset, no StyleCop default, no FSharpLint formatting preset; unpinned versions rejected: no floating version or head"
