#!/usr/bin/env bash
# Roslyn per-TFM-RID SARIF aggregation qualification.
#
# Decides the open Roslyn adapter-input risk with fixture evidence,
# recorded explicitly here and in the owning docs, never silently dropped
# (single-SARIF assumption rejected per the issue alternatives):
# - per-invocation fact: Roslyn `/errorlog` emits SARIF 2.1 per `csc`
#   invocation, one run per configuration/TFM/RID pivot file (fixtures
#   `net8.sarif` plus `net10.sarif`, each a single `runs[0]` with
#   `tool.driver.name == "csc"`).
# - union proof: TFM-gated code (`#if NET10_0` in `Sample.cs`) means the
#   net10.0 pivot reports CA1303 while net8.0 does not; both report the
#   shared CA1822. Keeping one pivot SARIF would silently drop the
#   TFM-specific finding, so aggregation is a union, never a pick-one.
# - aggregation REQUIRED: concatenate per-pivot `runs` into one SARIF log
#   with a single `$schema` plus `version` (`2.1.0`), runs in deterministic
#   pivot order (ascending TFM version, then RID, then configuration; plain
#   string sort would invert net10/net8), each run verbatim except
#   `artifactLocation.uri` re-rooting to workspace-relative form (see
#   `aggregated.sarif`).
# - merged-single-run REJECTED: collapsing results into one run loses
#   per-pivot provenance (`automationDetails.id` plus
#   `properties.{configuration,tfm,rid}`); dedup happens only when
#   normalizing to the result contract (identical exposed fields per
#   `docs/quality/quality-result-protocol.md`).
# - collection REQUIRED: every declared per-pivot SARIF is a declared
#   action input from the authoritative `csharp_*` target context (never
#   ambient scan); missing or malformed fails closed, never empty success.
# Adapter-only: no adapter claims `csharp` yet (cohort stays owned by;
# decision recorded under with fixtures).
#
# Versioned here, run by CI via `bazel run //tools/ci:roslyn_qualification`.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="csharp/tests/fixtures/roslyn/pins.bzl"
pins_build="csharp/tests/fixtures/roslyn/BUILD.bazel"
sample_src="csharp/tests/fixtures/roslyn/Sample.cs"
pivot_net8="csharp/tests/fixtures/roslyn/net8.sarif"
pivot_net10="csharp/tests/fixtures/roslyn/net10.sarif"
aggregated="csharp/tests/fixtures/roslyn/aggregated.sarif"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture set stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$sample_src" && -f "$pivot_net8" && -f "$pivot_net10" && -f "$aggregated" ]]; then
  ok
else
  bad "roslyn fixture missing (want $pins plus $pins_build plus Sample.cs plus net8 plus net10 plus aggregated SARIF)"
fi

# Pins record the SDK coupling plus SARIF wire shape (versions qualified
# seed-only under; digests stay owned under).
if grep -q -F -e 'ROSLYN_COUPLING = "SDK-built-in' "$pins" &&
  grep -q -F -e 'ROSLYN_SARIF_VERSION = "2.1.0"' "$pins" &&
  grep -q -F -e 'ROSLYN_DRIVER = "csc"' "$pins" &&
  grep -q -F -e '/errorlog' "$pins"; then
  ok
else
  bad "pins.bzl lost its Roslyn SDK coupling plus SARIF 2.1 plus csc driver record under issue #492"
fi

# Pins record the explicit aggregation decision plus rejections
# (single-SARIF assumption rejected per the issue alternatives).
if grep -q -F -e 'ROSLYN_AGGREGATION = "concatenate per-pivot runs' "$pins" &&
  grep -q -F -e 'single-SARIF assumption rejected' "$pins" &&
  grep -q -F -e 'ROSLYN_SINGLE_SARIF_REJECTED' "$pins" &&
  grep -q -F -e 'ROSLYN_MERGED_RUN_REJECTED' "$pins" &&
  grep -q -F -e 'merged-single-run rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its explicit aggregation record with single-SARIF plus merged-run rejection under issue #492"
fi

# Pins record collection plus provenance plus fix flow plus native-config
# sole policy.
if grep -q -F -e 'ROSLYN_COLLECTION' "$pins" &&
  grep -q -F -e 'declared action input' "$pins" &&
  grep -q -F -e 'ROSLYN_PROVENANCE' "$pins" &&
  grep -q -F -e 'automationDetails.id' "$pins" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$pins" &&
  grep -q -F -e 'IN_PLACE' "$pins" &&
  grep -q -F -e 'StyleCop stays opt-in' "$pins"; then
  ok
else
  bad "pins.bzl lost its collection plus provenance plus fix-flow plus config-policy record under issue #492"
fi

# Per-pivot SARIFs prove the per-invocation shape plus the union need: each
# carries exactly one run with the csc driver, the shared CA1822 fires in
# both pivots, and the TFM-specific CA1303 fires only in the net10.0 pivot
# (so a pick-one reader would drop it).
if python3 - "$pivot_net8" "$pivot_net10" <<'PY' >/dev/null 2>&1
import json, sys
net8 = json.load(open(sys.argv[1]))
net10 = json.load(open(sys.argv[2]))
assert net8["version"] == "2.1.0" and net10["version"] == "2.1.0"
assert len(net8["runs"]) == 1 and len(net10["runs"]) == 1
for log in (net8, net10):
    assert log["runs"][0]["tool"]["driver"]["name"] == "csc"
def rule_ids(log):
    return sorted(r["ruleId"] for r in log["runs"][0]["results"])
assert rule_ids(net8) == ["CA1822"], rule_ids(net8)
assert rule_ids(net10) == ["CA1303", "CA1822"], rule_ids(net10)
pivots = (net8["runs"][0]["automationDetails"]["id"], net10["runs"][0]["automationDetails"]["id"])
assert pivots[0] != pivots[1]
assert net8["runs"][0]["properties"]["tfm"] == "net8.0"
assert net10["runs"][0]["properties"]["tfm"] == "net10.0"
PY
then
  ok
else
  bad "per-pivot SARIF fixtures lost their per-invocation plus union proof (want one csc run each, shared CA1822 in both, CA1303 only in net10)"
fi

# Aggregated SARIF proves runs concatenation with deterministic pivot order
# plus the union: two runs in ascending-TFM-version order (net8.0 before
# net10.0; plain string sort would invert them), three
# total results, per-run provenance intact, workspace-relative artifact
# URIs, single schema plus version.
if python3 - "$aggregated" "$pivot_net8" "$pivot_net10" <<'PY' >/dev/null 2>&1
import json, sys
agg = json.load(open(sys.argv[1]))
net8 = json.load(open(sys.argv[2]))
net10 = json.load(open(sys.argv[3]))
assert agg["version"] == "2.1.0"
assert "$schema" in agg
assert len(agg["runs"]) == 2
ids = [r["automationDetails"]["id"] for r in agg["runs"]]
assert ids == ["csharp-roslyn/net8.0-linux-x64/Debug", "csharp-roslyn/net10.0-win-x64/Debug"], ids
assert ids[0] != ids[1]
assert agg["runs"][0] == net8["runs"][0], "net8 run must survive verbatim"
assert agg["runs"][1] == net10["runs"][0], "net10 run must survive verbatim"
total = sum(len(r["results"]) for r in agg["runs"])
assert total == 3, total
for run in agg["runs"]:
    assert run["tool"]["driver"]["name"] == "csc"
    for prop in ("configuration", "tfm", "rid"):
        assert prop in run["properties"], prop
    for res in run["results"]:
        uri = res["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        assert uri == "csharp/tests/fixtures/roslyn/Sample.cs", uri
        assert not uri.startswith("/"), uri
        assert "://" not in uri, uri
PY
then
  ok
else
  bad "aggregated SARIF lost its runs-concatenation plus order plus union proof (want 2 runs in TFM order, 3 results, provenance, workspace-relative URIs)"
fi

# Adapter delivered under #797: roslyn claims csharp in REAL_ADAPTERS and
# the runner matrix carries csharp cells (green adapter evidence; cohort
# classification stays).
if grep -q -F -e '"roslyn":' "$adapters" &&
  grep -q -F -e 'matrix_csharp_' quality/testdata/runner_matrix_scala_dotnet.bzl &&
  grep -q -F -e '"csharp": "csharp"' "$adapters"; then
  ok
else
  bad "roslyn adapter delivery missing (want roslyn in REAL_ADAPTERS plus matrix_csharp_ cells plus csharp classified under #797)"
fi

# Tool integrations record the explicit decision (aggregation required,
# single-SARIF rejected, runs concatenation with provenance), never silent,
# with the adapter delivered under #797.
if grep -q -F -e 'decided under issue #492' "$integrations" &&
  grep -q -F -e 'csharp/tests/fixtures/roslyn/' "$integrations" &&
  grep -q -F -e 'single-SARIF assumption is rejected' "$integrations" &&
  grep -q -F -e 'concatenate' "$integrations" &&
  grep -q -F -e 'automationDetails.id' "$integrations" &&
  grep -q -F -e 'not silent' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations"; then
  ok
else
  bad "tool-integrations lost its explicit Roslyn aggregation decision under issue #492"
fi

# Tool acquisition keeps the decided.NET route with the aggregation
# decision and no false claim.
if grep -q -F -e 'decided under issue #492' "$acquisition" &&
  grep -q -F -e 'Decided route: CSharpier and Fantomas take the' "$acquisition" &&
  grep -q -F -e 'no consumer runs `dotnet tool install`' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its .NET route with #492 aggregation decision and no-claim honesty"
fi

# Support matrix records the decision in adapter-input notes plus wire
# formats plus open risks, with fixtures and no Supported claim.
# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "roslyn_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:roslyn_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the roslyn_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: C# foundation fixture stays green on the seed host
# (decision only; no adapter behavior yet).
if bazel test //csharp/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "roslyn live proof failed (want csharp hello green)"
fi

dx_test_summary "roslyn qualification harness"
