ROSLYN_COUPLING = "SDK-built-in CA analyzers following the qualified .NET SDK, no separate acquisition"

ROSLYN_SARIF_VERSION = "2.1.0"
ROSLYN_ERRORLOG = "/errorlog SARIF 2.1 per csc invocation (one run per file)"
ROSLYN_DRIVER = "csc"

ROSLYN_AGGREGATION = "concatenate per-pivot runs into one SARIF log with single $schema plus version; union of results with per-pivot provenance; single-SARIF assumption rejected"
ROSLYN_RUN_ORDER = "deterministic pivot order: ascending TFM version then RID then configuration (net8.0-linux-x64 before net10.0-win-x64; plain string sort would invert net10/net8)"
ROSLYN_PROVENANCE = "per-run automationDetails.id plus properties.{configuration,tfm,rid}; artifactLocation.uri re-rooted to workspace-relative"
ROSLYN_COLLECTION = "every declared per-pivot /errorlog SARIF is a declared action input from the authoritative csharp_* target context; missing or malformed fails closed"
ROSLYN_MERGED_RUN_REJECTED = "merged-single-run rejected: loses per-pivot provenance and conflates TFM-gated diagnostics"
ROSLYN_SINGLE_SARIF_REJECTED = "single-SARIF assumption rejected: one pivot SARIF never claims full-target coverage"
ROSLYN_FIX_FLOW = "check-only with sandbox-apply-and-diff unified patches; IN_PLACE mutation rejected under sandboxing"

ROSLYN_CONFIG_POLICY = "SDK default analysis mode is the upstream built-in default, StyleCop stays opt-in"

ROSLYN_FIXTURE_PIVOT_NET8 = "csharp-roslyn/net8.0-linux-x64/Debug"
ROSLYN_FIXTURE_PIVOT_NET10 = "csharp-roslyn/net10.0-win-x64/Debug"
ROSLYN_FIXTURE_SHARED_RULE = "CA1822"
ROSLYN_FIXTURE_TFM_SPECIFIC_RULE = "CA1303"

CSHARP_FIXTURE_HELLO = "//csharp/tests/fixtures/hello:hello_test"

ROSLYN_REJECTED = "single-SARIF assumption rejected: must record; merged-single-run rejected; ambient SARIF discovery rejected; IN_PLACE patching rejected"
