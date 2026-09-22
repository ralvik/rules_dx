# Audit, Update, And Bazel Commands

## `dx bazel`

```text
dx bazel <bazel arguments...>
```

Invokes the repository's Bazelisk-compatible `bazel` launcher with arguments
unchanged. The launcher reads the committed `.bazelversion`. `dx bazel` does not
perform scope resolution. This is the advanced-user escape hatch. Everything
after the `bazel` word forwards verbatim to the launcher, even tokens that look
like dx options, so dx globals must precede it (`dx --dry-run bazel ...`);
a dx-owned option before the command word is rejected rather than forwarded.

## `dx audit`

```text
dx audit [--here] [security|license] [scope ...] [--report <format>=<destination> ...]
```

Implementation status: the audit/update policy below is accepted. Command dispatch
and request planning are implemented (the `dx_audit`/`dx_update` planning gates
plus `dx audit`/`dx update` dispatch — `--dry-run` plans the request and exits `0`).
Live `dx audit` executes qualified auditors per family over resolved scopes with
per-family reporting and SARIF plus SPDX 2.3 JSON through the shared `--report`
contract, pinned by fixtures in `dx_audit` and `dx_cli`. Live `dx update` executes
resolver-owned backends per dependency set with independent-set continuation
and per-set reporting as specified in `dx update` below.

Bare `dx audit` runs both families. `dx audit security` runs secrets plus
dependency-vulnerability analysis only; `dx audit license` runs license-policy
analysis only. Family selection composes with the normal scope resolution below;
it does not change scope defaults. Pass `--here` (`--cwd` alias, optionally
after the family) for the current directory tree instead of `//...`.

`dx audit` applies the non-mutating audit aspect and any approved ecosystem dependency-
audit integrations to the selected Bazel scope. It has two families, `security`
(secrets plus dependency-vulnerability analysis) and `license` (dependency
license-policy analysis); it is not an umbrella
for lint, formatting, tests, or builds. Gitleaks is the V1 secrets integration
(Trufflehog wont-fix, issue #629: one pinned tool, silent swap rejected),
run as `<hermetic-gitleaks> detect --source . --report-format sarif --report-path <temp>`
with `--redact` and `--exit-code 2`, using built-in defaults unless
`.gitleaks.toml` is committed (explicit `--config` then pins it). The binary
is the pinned `@dx_tools//:gitleaks` standalone artifact (v8.30.1 on all five
required hosts, checksums in `quality/artifacts/gitleaks.*.bzl`): `argv[0]` is
always the absolute declared path resolved from `DX_GITLEAKS_BIN`, never a bare
`gitleaks` ambient `PATH` lookup, and a missing or relative tool fails closed
with an actionable diagnostic. The child spawns cleared: exactly `TMPDIR` (the
per-run temp directory) reaches Gitleaks, `PATH` is never set, and
`GITLEAKS_CONFIG`/`GITLEAKS_CONFIG_TOML` plus every other parent variable are
never inherited, so configuration flows only through explicit `--config`, the
committed `.gitleaks.toml`, or built-in defaults and ambient values cannot
inject rules. Detection flags are unchanged. Platform evidence: SARIF
pass/fail triage is host-independent and pinned by fixtures in
`dx_audit::secrets` plus `dx_cli::exec::audit`; per-host bytes are pinned by
checksums plus archive members in `quality/artifacts/gitleaks.*.bzl`
(regenerated from upstream, seed-host bytes fetched and verified), with
execution-platform selection supplying the matching host artifact and no
ambient fallback. SARIF output
is triaged for findings-versus-error: exit `0` with empty results is clean,
exit `1` with results is findings, exit `1` with no results or malformed SARIF
is incomplete, and any other exit or launch failure is incomplete. Summaries
render only rule IDs and counts, never secret values: triage rebuilds each
finding message from the rule ID plus the artifact path only and ignores
SARIF `message.text`, fingerprints, snippets, fixes, and properties, so even
an unredacted report cannot leak values (issue #629, pinned in
`dx_audit::secrets` plus `dx_cli::exec::audit`). The
`secrets` policy-family mapping is its own semantic class with SARIF and secret-value redaction,
reconciled with source-class applicability in
[Quality Sources](../../quality/quality-sources.md).
Dependency-vulnerability matching runs locally per set (Cargo, npm, Maven,
NuGet, Go) with no lockfile or inventory upload. Npm audits every
present lock shape (`pnpm-lock.yaml`, `package-lock.json`, `yarn.lock`);
absent shapes are skipped, so a pnpm-only workspace never fails for a
missing sibling lock. Go reads the
`go_deps.from_file` module lock (`third_party/go/go.mod`); `go.sum`
carries hashes only and is never an audit input.
With no scope, audit selects `//...`, while each audit adapter remains responsible
for its declared applicability. Source-audit adapters use the same provider-class,
adapter-class, derived workspace-policy, and capability intersection as convergence stages,
followed by explicit native-config exclusions. Dependency-audit integrations instead use their
authoritative ecosystem target/provider contracts and never infer source classes.

For target-scoped dependency audits, select the targets' owning dependency sets and audit their
complete standard locks or equivalent resolved dependency files, including dependencies not used
by those particular targets. Do not filter findings to a target's resolved package closure.
Shared owning sets are audited once per distinct audit context; unrelated dependency sets are not
included merely because they are in the same repository. Bare `dx audit` remains repository-wide.
This dependency-set scope does not broaden source-audit selection. Target-to-owner mappings
use the approved `dx_update` set registry, so audit and update agree on owning sets.

Dependency audits automatically refresh applicable vulnerability advisory data through supported
upstream tooling when invoked; a separate manual refresh is not the default workflow. Supply an
identified advisory snapshot as an input to Bazel-owned analysis, so results and cache identity
reflect the data actually analyzed rather than an untracked live database inside the audit action.
Acquisition/cache updates do not change application dependency versions, manifests, lockfiles, or
selected environment/codegen projections. Auditors remain pinned tools; advisory freshness does not
authorize automatic tool-version upgrades. If required advisory refresh fails, fail the audit and
report that current data could not be obtained. Do not fall back to a stale snapshot for the affected
dependency audit or report that dependency set as clean. V1 reads identified snapshots from
`.dx/advisory/<set>.json` plus identity `.dx/advisory/<set>.meta.json` (`url`, `sha256`,
`retrieved_at`), derived via supported upstream database-download tooling (per-set OSV GCS
`all.zip` fetched by HTTPS GET with no inventory in the request, such as `osv-scanner --offline`
with a local DB; the OSV query API discloses the inventory and never satisfies this contract),
with 24h same-day freshness and refresh-failure mapping pinned in `dx_audit::advisory`; a missing,
invalid, or stale snapshot fails with `advisory_refresh_failed`, never clean and never a stale
fallback; live CLI performs no network fetch.

Airgapped workspaces populate the same `.dx/advisory/` inputs by
copying the vendored advisory mirror instead of fetching (see
[Offline Bootstrap](../../deploy/offline-bootstrap.md#vendored-advisory-mirror)):
mirror identities carry a `file://` URL naming the vendored source
while upstream identities keep their `https://` database-download URL,
and both shapes enforce the same `sha256` byte binding plus the same
same-day freshness gate with the same fail-closed mapping.

Keep dependency inventories out of external vulnerability services. Download applicable advisory
databases and match packages against the identified snapshots within Bazel-owned analysis; do not
upload lockfiles or send dependency package names and versions through query parameters, request
bodies, or auditor telemetry. Package-specific advisory requests that disclose the inventory are
not an alternative to local matching. V1 matches locally against the supplied snapshot bytes
with no network access after inputs are acquired; a query-only upstream service does not satisfy
this contract. This restriction concerns
vulnerability services and does not change separately configured Bazel remote execution/cache
boundaries for declared analysis inputs.

Audit supports SARIF 2.1.0 reports through the shared `--report` contract (plus SPDX 2.3 JSON
for license, see below). SARIF run shape is pinned under issue #632: one deterministically
ordered run per executed tool (`gitleaks`, `vuln` for security; `license` for license) with
stable driver/rule IDs and workspace-relative artifact URIs, severity mapped to
`note`/`warning`/`error`, locations path-only (no byte ranges, hence no regions), empty runs
kept, and deterministic ordering independent of finding order. Partial
collection marks every run's invocation unsuccessful (`executionSuccessful=false`) while
retaining validated findings. Partial reports stay non-authoritative: they are marked for
archival and debugging and must not be uploaded as a replacement scan; authoritative upload
is gated on `results_complete=true`.

If a selected dependency cannot be assessed by the qualified auditor, fail audit as incomplete and
identify the dependency and assessment limitation. Examples include unsupported Git revisions or
unidentified private packages. Retain validated findings from assessed dependencies, but do not
claim complete coverage or treat an unassessed dependency as having no known vulnerabilities.
A recognized, assessable package with no matching advisories is a different result and is not
itself a coverage failure. Advisory-specific risk acceptance does not waive missing assessment.
An empty findings list alone is not evidence that every selected dependency was assessed.
Per-ecosystem dispositions are wont-fix, pinned by fixtures in `dx_audit::vuln` plus
`dx_audit::locks` (issue #584): Git revisions stay incomplete (auditor-owned; SHAs carry no
OSV version identity and SHA-to-version mapping needs a network resolver forbidden by the
offline contract; `paket.lock` `GIT` entries report incomplete rather than dropping),
unidentified private packages stay incomplete (auditor-owned; no upstream identity by
definition, callers mark `is_private` explicitly). Npm lock shapes plus
git plus private handling is implemented (issue #627, pinned in
`dx_audit::locks` plus `dx_audit::vuln` plus `dx_cli::exec::audit`):
`pnpm-lock.yaml` (multi-document env plus project graphs merged,
`link:`/`file:` workspace members skipped, git `resolution: {type:
git}` with `repo`/`commit`, host-archive tarballs, and git-shaped
keys/versions flagged incomplete), `package-lock.json` (v1/v2/v3
`packages:` plus legacy `dependencies:`, `link`/`file:` members
skipped, git-shaped `version`/`resolved`/`from` flagged incomplete),
and `yarn.lock` v1 (`file:`/`link:`/`portal:` skipped, git-shaped
headers/versions/`resolved` flagged incomplete); private entries are
byte-identical to public ones in every npm lock shape, so callers mark
`is_private` explicitly and matching fails those as incomplete, never
clean. Maven range narrowing is
implemented (issue #623, pinned in `dx_audit::vuln`), as is NuGet range narrowing
(issue #624, pinned in `dx_audit::vuln`), as is Go `go.mod` wiring plus
`v`-prefix range narrowing (issue #626, pinned in `dx_audit::locks` plus
`dx_audit::vuln`), as is Go pseudo-version plus `+incompatible` flavor
(issue #679, pinned in `dx_audit::vuln`), as are Cargo/npm semver edges
(issue #625, pinned in `dx_audit::exception` plus `dx_audit::vuln`), as is
license exception narrowing (issue #631, pinned in `dx_audit::license_policy`
plus `dx_audit::vuln` plus `dx_cli::exec::audit`).

Report known vulnerabilities whether or not a fixed version is available, and apply the same
severity threshold and failure policy in both cases. Lack of a fix must not suppress a finding,
downgrade its severity, or exempt it from failure. Preserve upstream remediation information when
available, without treating a dependency-version upgrade as an automatic source fix or mutating
dependencies during audit. Advisory scope uses upstream Cargo-flavor semver for Cargo,
npm-native ranges for npm (issue #625), Go via `v`-prefix normalization plus
pseudo-version ordering (issue #626 plus issue #679), Maven-native ordering
plus intervals
for Maven (issue #623), and NuGet-native ordering plus
intervals for NuGet (issue #624), pinned in `dx_audit::vuln`. Cargo scopes accept
ranges (`>=1.2.0, <2.0.0`), carets, tildes, and `*`; bare versions are caret
shorthand (so `1.2.0` matches `1.2.1`) and `||` plus hyphen stay invalid,
failing closed to no-match. Npm scopes accept `||` unions, hyphen ranges
(`1.2.3 - 2.3.4`, partial ends narrow below the next line), space- or
comma-separated sets (`>=1.2.7 <1.3.0`), `v` prefixes, `x`/`*` wildcards and
partials, carets, tildes, and bare versions (full bares are exact, partial
bares are bounded); prereleases match only beside a same-tuple prerelease
comparator and malformed scopes fail closed to no-match. Go scopes normalize
one leading `v` on version tokens in both the advisory scope and the locked version
(`>=v1.0.0, <v2.0.0` matches `v1.5.0`), then Cargo-flavor ordering without the
Cargo prerelease gate (issue #679): pseudo-versions
(`v0.0.0-20250930140053-2eb4fccefb52`,
`v1.2.4-0.20240101120000-abcdef123456`) match bare ranges they fall inside
(`>=v1.0.0, <v2.0.0` covers `v1.2.4-0.20240101-abcdef`; `*` covers pseudos;
`=v1.2.4` stays exact and `>=v1.2.4` still excludes its pre-release pseudos),
`+incompatible` suffixes ride build metadata ignored for precedence
(`=v2.0.0` matches `v2.0.0+incompatible`), and malformed scopes fail closed
to no-match; no Go-aware crate (preprocessing plus the gate bypass suffices).
Maven scopes accept bare versions
(Maven equality, so `1.0` matches `1.0.0`) and bracketed intervals (`[1.0,2.0)`, `(,1.0]`,
`[1.5,)`, `[1.0]`, unions like `(,1.0],[1.2,)`), with inclusive `[`/`]` versus exclusive
`(`/`)` bounds; malformed scopes fail closed to no-match. NuGet scopes accept bare versions
(NuGet equality, so `1.0` matches `1.0.0` but not `1.5.0`; `[1.0,)` spells the `>=` minimum)
and bracketed intervals (`[1.0,2.0)`, `(,1.0]`, `[1.5,)`, `[1.0]`), with inclusive `[`/`]`
versus exclusive `(`/`)` bounds; floating `*`, unions, and `(1.0)` single-exclusive stay
invalid and malformed scopes fail closed to no-match. Crate reuse beyond
`cargo-lock`, `osv`, `semver`, `serde_json`, `yaml_serde`, `toml`, `url`,
`hex`, and `procfs` stays as evaluated under issue #750: Maven/NuGet
ordering plus intervals, npm partial/hyphen/`||` narrowing, Go
`v`-prefix plus pseudo-version ordering, and yarn/pnpm/`package-lock`/
paket/`go.mod` text shapes keep their hand-rolled parsers with per-site
reason comments in `dx_audit` (no stable upstream crate for those exact
fail-closed semantics). Vulnerability exceptions use
the same per-set narrowing. License exceptions narrow the same way
(issue #631, pinned in `dx_audit::license_policy`): package plus owning
set plus license identity with the finding version inside the exception
scope under that set's upstream semantics; out-of-range versions do not
inherit acceptance.

Known applicable vulnerabilities with no severity rating fail audit by default. Report the
upstream advisory severity as unknown text rather than inventing a rating or silently
treating missing metadata as non-blocking. The normalized diagnostic level stays in the
closed `info|warning|error` set owned by the [Output Protocol](../output-protocol.md#diagnostic).
A valid explicit risk-acceptance exception may exempt the finding from failure while retaining its
visibility. Upstream severity text is preserved (`unknown` when unrated) and normalized to
the closed `info|warning|error` set (`critical|high` to `error`, `medium|low` to `warning`,
missing or unrecognized to `error`); `--fail-on` thresholds apply.

Explicit risk-acceptance exceptions may exempt particular vulnerability findings from failure.
Each exception must identify the advisory and affected dependency in its owning dependency scope
and specify the accepted dependency version or bounded version range, with an explanatory reason.
Versions outside that selection are not accepted even when the advisory and dependency match;
extending acceptance requires an explicit reviewed configuration change. Use upstream version and
advisory identity semantics, including qualified alias matching, rather than a private version solver.
Prefer upstream-native configuration; do not silently broaden
a dependency-specific exception to every package or advisory. Exceptions do not waive advisory
refresh, acquisition, analysis, or report-collection failures, or exempt unrelated findings.
Accepted vulnerabilities remain visible in normal audit output and reports, explicitly marked as
accepted/suppressed with their explanatory reason. Preserve their vulnerability identity and
severity; risk acceptance excludes the finding from the failure decision, not from visibility,
and must not present it as fixed. Vulnerability exceptions have no separate native
configuration; identity matching is advisory plus package plus owning set with
upstream version-range narrowing. Lack of a fix alone is not an implicit exception.

Every vulnerability risk-acceptance exception requires an expiration date. Missing, invalid, or
expired dates fail validation; an expired exception no longer exempts its finding from the normal
failure policy. Renewal requires an explicit reviewed configuration change, not automatic extension
by the auditor. Date syntax is strict ISO-8601 UTC `YYYY-MM-DD` with inclusive expiry
(expiring today is expired), evaluated at audit time. An earlier cached acceptance must not allow a later
audit invocation to pass after expiry. This requirement applies to vulnerability risk acceptance,
not to the separate declared-dependency usage exceptions.

Obsolete risk exceptions fail validation even before expiration. An exception must still match an
applicable vulnerability in its complete owning dependency set against the current advisory
snapshot; removing the dependency or upgrading beyond the affected versions makes it obsolete.
Report the obsolete entry for explicit removal, without deleting it automatically. Do not infer
obsolescence from failed/incomplete analysis or from an unrelated owner being outside the selected
audit scope. Obsolescence is identity match over advisory plus package against current
findings, with version-range narrowing for applicability.

Remaining audit integration details follow these established defaults: fail incomplete assessment,
keep matching local to declared advisory snapshots, preserve truthful visible findings, and permit
only narrow, explained, version-scoped, expiring risk acceptance. Prefer the simplest conforming
upstream integration.

Aggregate exit status is pinned in `dx_audit::outcome`: a fully assessed run with no unexempted
findings exits `0`; unexempted findings or incomplete assessment exit `1`, with the
findings-versus-error split recorded in the report rather than the code. Family results arrive
in canonical security-first order. Usage errors stay exit `2` at the CLI layer per the common
contract. Per-family reporting rides text plus JSON `notice`/`error` events with
`command_finished`, and aggregate exit-code selection rides `dx_audit::outcome`.

### License family (`dx audit license`)

Use per-root (per-dependency) attribution over
conservative whole-lock strictness, without silently narrowing complete-lock audit coverage.
Per-ecosystem license identities are Cargo via `cargo-bazel-lock.json`, npm via
`package-lock.json` `license` fields (both `packages:` and legacy `dependencies:` shapes;
`pnpm-lock.yaml`/`yarn.lock` carry no license), and Maven/NuGet/Go via the committed
`[[inventory]]` table in `licenses.toml` (also the notice-text source for all sets, and the
override for Cargo/npm where it matches); uninventoried packages stay `UNKNOWN`.
Policy-table loading reads committed `licenses.toml` (default table when absent, matching
the example below) and SPDX 2.3 JSON renders one document per invocation as specified.

The license family reuses security-audit scope mechanics (default `//...`,
per-target owning dependency sets, complete-lock coverage, local matching with
no lockfile upload) and exception lifecycle (version-scoped with upstream
version semantics, reasoned, expiring with ISO-8601 UTC dates evaluated at
audit time, obsolete only when no applicable finding remains, always visible). An upgrade
within an exception's bounded version range retains acceptance while the exception still
matches the finding and remains otherwise valid; upgrading alone does not invalidate it.
Its report format is SPDX 2.3
JSON via the shared `--report` contract, pinned under issue #632: one
document per invocation (never per package, set, or root) with `SPDX-2.3`,
`CC0-1.0`, `SPDXRef-DOCUMENT`, name `dx-audit-license`, and an
invocation-unique namespace; packages sorted by ID with `licenseConcluded`/
`licenseDeclared`, `NOASSERTION` copyright, and single purl `externalRefs`
(package URLs via the purl spec); `DESCRIBES` relations
from each audited root first (sorted), then `CONTAINS` where the lock graph is
known (V1 emits none: no lock-graph edges projected yet). Live emission is
pinned by goldens in `dx_audit::spdx` plus `dx_cli::exec::audit` and stays
deterministic over input order; partial SPDX stays non-authoritative like
SARIF above, gated on `results_complete`.

Two distribution tiers only:

- `distributed`: release roots that leave the company (binaries, images,
  packages, SaaS offerings). Strict table: allow MIT/Apache-2.0/BSD/ISC,
  review LGPL/MPL/EPL, deny GPL/unknown plus everything in `[policy.blocked]`.
- `internal`: everything else, including dev tools and internal-only binaries.
  Inventoried in the SBOM and never fails on the allow/review/deny table —
  except `[policy.blocked]`, which fails in both tiers. A blocked license still
  needs a versioned exception with reason and expiry to pass anywhere.

The committed root file is TOML (dedicated `licenses.toml`, matching the
dedicated-config rule). There are no per-directory policy files and no local
overlay that relaxes license policy. A distributable is marked internal by
listing its label under `[distribution.internal]`; unlisted distributables
default to `distributed` (fail closed). The name `distributed` is deliberate:
distribution is the legal trigger, while `ship` is slang. Promoting an
internal root to distributed re-qualifies it under the strict table on the
next audit.

As-built in this repository ([`licenses.toml`](../../../licenses.toml),
issue #643): `//cli/cli:dx` is the one marked distributed root (the release
binary); the `//...` dogfood scope stays internal-only because no release is
cut. Exercise the strict gate on the existing customer flow with
`bazel run //cli/cli:dx -- audit license //cli/cli:dx`.

In `distributed`, an unlisted SPDX identity or a final `review` outcome fails unless explicitly
approved in committed policy. A matching, reasoned, version-scoped, unexpired exception can
approve a finding without hiding it. Merely placing an identity in `review` is not approval.
The `internal` inventory-only rule remains unchanged except for `[policy.blocked]`.

```toml
# Global table: each listed SPDX identity belongs in exactly one list. `blocked` is
# evaluated in both tiers (network-trigger and non-open licenses that are
# never silently acceptable); the rest only gates `distributed`.
[policy]
blocked = ["AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0"]

[policy.distributed]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unlicense"]
review = ["LGPL-2.1-only", "LGPL-2.1-or-later", "LGPL-3.0-only", "LGPL-3.0-or-later",
          "MPL-2.0", "EPL-2.0", "CDDL-1.0"]
deny = ["GPL-2.0-only", "GPL-2.0-or-later", "GPL-3.0-only", "GPL-3.0-or-later"]

# Per-set adjustments merge additively; conflicts with the global table fail validation.
[policy.sets.npm-root]
review = ["Unicode-3.0"]

[distribution]
distributed = ["//services/payments:image", "//cli:dx"]
internal = ["//tools/internal-admin:binary"]

[[exception]]
package = "some-copyleft-lib"
set = "cargo"
license = "GPL-3.0-only"
versions = ">=1.2.0, <2.0.0"
reason = "Legal approved for internal fork; re-review on major bump."
expires = "2027-03-01"

# Per-ecosystem identities plus per-package notice texts: the first entry
# matching package plus set with an in-scope version (upstream semantics
# per set, like exceptions) supplies the SPDX identity; `text_present`
# records whether the package archive delivered LICENSE*/NOTICE* words
# (absent means no words, fail closed). Missing entries stay UNKNOWN.
[[inventory]]
package = "react"
set = "npm"
license = "MIT"
versions = "18.2.0"
text_present = true

[[inventory]]
package = "junit:junit"
set = "maven"
license = "EPL-1.0"
versions = "4.13.2"
text_present = true

[[inventory]]
package = "FSharp.Core"
set = "nuget"
license = "MIT"
versions = "10.1.201"
text_present = true

[[inventory]]
package = "github.com/google/go-cmp"
set = "go"
license = "BSD-3-Clause"
versions = "v0.6.0"
text_present = true
```

SPDX expression evaluation follows boolean math over the allow/review/deny
lattice, with `blocked` acting as deny in both tiers:

- `A OR B` passes if any disjunct is allowed (the distributor chooses the
  license); otherwise review if any disjunct needs review; otherwise denied.
  `MIT OR AGPL-3.0-only` therefore passes by choosing MIT.
- `A AND B` is denied if any conjunct is denied (all terms must be satisfied);
  otherwise review if any conjunct needs review; otherwise allowed.
- `WITH <exception>` expressions require approval of the complete expression verbatim
  wherever license-policy approval is required. Allowing the base license alone does not
  approve the expression. A `review` or deny listing is not approval; the normal tier,
  blocked-policy, and explicit-exception rules still apply.
- `UNKNOWN` or unparseable license text is denied in `distributed`,
  inventoried in `internal`.

Why license texts are separate inputs: lock metadata says *which* license a
package claims; MIT/BSD/Apache-2.0 legally require reproducing *the words*
(copyright notice plus text, including compounds like `MIT OR Apache-2.0` where any
member requires). Those words ride per-package `text_present` in `[[inventory]]`
(committed curator data; the future declared-Bazel-inputs delivery keeps the same
per-package shape so NOTICE aggregation stays hermetic and cached). A package whose
license requires reproduction but ships no text reports `missing-notice-text`, which
fails in `distributed` unless excepted and is inventoried in `internal`. The
packaging pipeline that assembles the aggregated NOTICE from these
already-validated inputs is `notice_bundle` in `deploy/release/notice.bzl`
(hermetic `notice_gen`, byte-identical rebuilds, `missing-notice-text` fails
the action; verified by `notice_verify_files` plus `dx_verify --notice`
before install, and signed alongside the SBOM bundle via
`//deploy/release:signing_demo`); see the
[release runbook](../../deploy/release-runbook.md#steps) for the release evidence.

Validation (all fail the audit, none auto-repair):

- Unknown label under `[distribution]` fails as `unknown_distribution_root`.
- A license ID in more than one policy list fails.
- An exception with no applicable finding fails as obsolete, like vuln
  exceptions (identity over package plus owning set plus license);
  missing, invalid, or expired dates fail; out-of-range versions
  do not inherit acceptance (per-set upstream narrowing, issue #631).
- An inventory entry with an empty package, set, license, or versions fails;
  unknown inventory keys fail as invalid TOML. Out-of-range versions never
  inherit the entry (missing entries stay `UNKNOWN`, fail closed in
  `distributed`).

## `dx update`

With no selection, `dx update` updates all supported dependency sets in the repository,
independent of the current working directory. It does not restrict updates to the containing
ecosystem workspace. Dependency-set discovery and selection use approved Bazel integrations,
not a CLI filesystem scan or a second dependency graph.

V1 supports selecting dependency sets (ecosystem workspaces/lockfiles) and individual packages
within selected sets. Delegate package selection to the upstream updater; necessary transitive
changes remain permitted under its resolver semantics. Package selection is not a guarantee that
only one lockfile entry changes, nor permission to silently substitute an update of the entire set
when the upstream integration cannot support the requested selection. Selector syntax and
ecosystem package-identity mappings are implemented in `dx_update::selector` and pinned by
unit tests: `cargo`/`npm`/`maven`/`nuget`/`go` select sets, `set:package` selects packages
(`maven:group:artifact` for Maven), and Bazel labels/patterns/files/dirs resolve to owning
sets via the approved prefix table (bare `//...` and `MODULE.bazel` select all sets).

Per-set selective support is decided in [ADR 0024](../../decisions/0024-selective-update.md)
with fixtures in `cli/update/tests/fixtures/selective_update/`:

| Set | Selective (`set:package`) | Full (`set`) |
| --- | --- | --- |
| npm | Supported (`bazel run @pnpm//:pnpm -- update [<pkg>...]`) | Supported |
| cargo | Wont-fix (`unsupported`; use `dx update cargo`) | Supported (`CARGO_BAZEL_REPIN=1` repin) |
| maven | Wont-fix (`unsupported`; use `dx update maven`) | Supported (`REPIN=1` pin) |
| nuget | Wont-fix (`unsupported`; use `dx update nuget`) | Supported (`paket2bazel` regen) |
| go | Wont-fix (`unsupported`; pinned module lock tracks Gazelle; widen via `dx bump`) | No-op success (pinned module lock) |

When a set is both fully and package selected, the full update wins.

Cargo per-crate (`cargo:<crate>`, e.g. `cargo:anyhow`) is wont-fix
(issue #633, fixtures in `cli/update/tests/fixtures/selective_cargo/`):
the approved `crate_universe` repin has no per-crate flag, so the
selector parses the crate name then fails closed as `unsupported` with
the `dx update cargo` hint and never silently substitutes a full
update; private `cargo update -p` stays rejected as a private resolver.

NuGet per-package (`nuget:<id>`, e.g. `nuget:FSharp.Core`) is wont-fix
(issue #635, fixtures in `cli/update/tests/fixtures/selective_nuget/`):
the approved `paket2bazel` regen has no per-id flag, so the selector
parses the id then fails closed as `unsupported` with the
`dx update nuget` hint and never silently substitutes a full update;
private `paket.lock` surgery stays rejected as a private resolver.

Maven per-artifact (`maven:group:artifact`, e.g. `maven:junit:junit` and
`maven:org.junit.jupiter:junit-jupiter-api`) parses but fails closed as wont-fix
(issue #634, fixtures in `cli/update/tests/fixtures/selective_maven/`):
`rules_jvm_external` offers no per-artifact pin target, so the whole-lock
`REPIN=1 bazel run @maven//:pin` stays the only approved updater.

Go per-module (`go:<module-path>`, e.g. `go:github.com/google/go-cmp/cmp`)
is wont-fix (issue #636, fixtures in
`cli/update/tests/fixtures/selective_go/`): the main workspace has no
`go.mod` by design and the pinned `go_deps.from_file` module lock
(`third_party/go/go.mod` plus `go.sum`) tracks Gazelle, so full stays an
intentional no-op success with no launch and selective parses then fails
closed as `unsupported` with the `dx bump gomod:<module> <version>` hint,
never silently substituting the no-op; private `go get` plus `go mod tidy`
stays rejected as a private resolver.

`dx update` updates selected dependencies to the newest versions permitted by the project's
declared requirements and authoritative ecosystem resolver, through approved Bazel integration.
It refreshes standard locks or equivalent resolved dependency files without widening or replacing
declared version requirements. This follows upstream within-constraint update semantics, such as
`uv lock --upgrade`, rather than rewriting requirements to track the latest release unconditionally.
Exact requirement pins remain constraints; old versions recorded only in a lockfile may be updated.
Prerelease eligibility follows the upstream resolver and project configuration, not a private policy.

Git dependencies follow upstream update semantics: a declared branch may advance its locked commit,
while explicit commit pins and declared tags remain unchanged. Do not rewrite a branch, tag, or
commit requirement to track another reference or select a newer tag. The upstream resolver owns Git
resolution and lockfile updates; this does not authorize inspecting the consumer's Git worktree.
Moved-tag behavior follows the upstream resolver with no private policy; requirement shapes are
pinned in `dx_update::semantics` (`GitRequirement`) and unit-tested.

Transitive dependencies remain governed by upstream resolution; the command does not force every
transitive package to its newest release regardless of compatibility. A newer release outside the
declared requirements is not an update failure. Resolution failures or unsupported update operations
must be reported rather than claimed as successful updates.
A successful dependency update is not a guarantee that application code still builds or passes tests.

Failure in one dependency set does not stop updates to independent selected sets. Preserve successful
changes, report each failure, and return an overall failure status if any selected update fails.
Do not run operations that depend on a failed update; report them as blocked, not successful.
Atomicity is per set, never repository-wide: each backend success commits that set's locks
immediately and a later failure or interruption keeps preceding successes with no automatic
rollback. Independence must follow the approved
upstream integration: sets sharing a lockfile or resolver workspace cannot be treated as independent
merely because they have different Bazel labels.
This is the explicit update exception to [common fail-fast handling](../cli-contract.md#exit-status),
not permission to run dependents of failed operations or introduce a private scheduler.
Aggregate exit-code selection is pinned in `dx_update::report`: a run with no failed selected set
exits `0`; any failed set fails the invocation overall with exit `1`, following the report's
`overall_failure` verdict (blocked without failure is not a failure). Per-set detail rides the
per-set report, never a per-set code. Backend operation boundaries are pinned in
`dx_update::backend` (Cargo `CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`, npm
`bazel run @pnpm//:pnpm -- update`, Maven `REPIN=1 bazel run @maven//:pin`, NuGet
`paket2bazel` regeneration, Go pinned no-op) and per-set success/failure/blocked reporting rides
text plus JSON `notice`/`error` events with `command_finished`. Continued updates do not imply
parallel execution. The v1.0 absence of file events was wont-fix (issue #586,
resolver-owned by `dx_update::backend`, pinned by fixtures in
`cli/update/tests/fixtures/update_events/` plus `cli/cli/src/exec/update.rs`); minor 1.1
adds the backend committed-change manifest (`dx_update::manifest`, validated against
`dx_update::sets::SetId::locks`, pinned by fixtures in
`cli/update/tests/fixtures/correlation_manifest/` plus `cli/cli/src/exec/update.rs`,
issue #811): validated file changes project to paired `change` plus `applied` `mutation`
events grouped under `update:<set>` before that set's terminal report, empty or absent
manifests emit no file events preserving the v1.0 contract, and Git scan/BUILD parse/rerun
inference stays rejected, so per-set
`notice`/`error` plus `command_finished` stays the complete event contract when no manifest
supplies deltas, with sorted per-set order,
`results_complete=true` on live terminal reports, an `update_recovery` notice plus
`dx: update_recovery:` text hint on failure carrying the idempotent retry (`dx update`
with the failed/blocked sets) and the manual restore (`git checkout --` with the kept
locks, run by the operator when version-controlled; `dx` never runs Git), and interrupted
runs keeping preceding per-set events true with nothing for sets not yet attempted and the
same retry shape for the unattempted sets. Recovery planning lives in `dx_update::recovery`
and is pinned by fixtures in `cli/update/tests/fixtures/update_rollback/`. See the
[Output Protocol](../output-protocol.md#mutation) for the full contract.

Invoking `dx update` authorizes immediate application without an interactive confirmation prompt or
separate acceptance flag, in both terminal and noninteractive use. This does not bypass separate
license-consent or mutation-safety requirements.

The command introduces no `dx` lockfile or dependency resolver.
Every changed file and invoked operation must be attributable to the
underlying updater. Ordinary builds and editor activity do not initiate dependency-version upgrades.
Supported ecosystem mappings are the five sets in `dx_update::sets` (Cargo, npm, Maven, NuGet, Go
with manifests/locks pinned there); selective-update syntax is `set:package` in
`dx_update::selector` (supported for npm, reported `unsupported` for Cargo/Maven/NuGet/Go rather than
silently widened, decided in [ADR 0024](../../decisions/0024-selective-update.md) with fixtures in
`cli/update/tests/fixtures/selective_update/` plus per-set evidence in
`cli/update/tests/fixtures/selective_cargo/` under issue #633 plus
`cli/update/tests/fixtures/selective_nuget/` under issue #635 plus
`cli/update/tests/fixtures/selective_maven/` under issue #634); non-registry handling is upstream-owned (Git branches may advance, tags/commit
pins stay, path dependencies are upstream no-ops); upstream operation/report mappings are pinned
in `dx_update::backend` and unit-tested.

### `dx update --check` (preset stale gate, accepted)

```text
dx update --check
```

Non-mutating preset freshness gate: regenerates the vendored
`tools/bazelrc/preset.bazelrc` fragment in memory, diffs against the
checked-in file, and exits `0` clean / `1` stale (copies the
[`dx generate --check`](generate.md#modes) exit contract). Selectors are
ignored; only the fragment is checked. Root `.bazelrc` lines duplicating
preset flags fail operationally (`update_failed`) like the
`preset.update -- --verify-only` collision gate. Text prints the unified
flag diff on stale; JSON emits `command_started` (`mode: check`),
`error`, and `command_finished`. `--dry-run` plans without reading.
Default `dx update` always regenerates the fragment atomically first
(preserving overrides, creating `tools/bazelrc/` for consumers), then
runs the selected dependency backends. See the
[preset update loop](../../contributing/local-workflows.md#preset-update-loop)
for the regen-and-review workflow.

## `dx bump`

```text
dx bump <set:package> <version>
```

Explicit widen-one-requirement operation (issue #260), separate from `dx update`.
It rewrites exactly one declared requirement in the working copy, never a whole
set and never a batch. `dx update` keeps its never-rewrites contract
(`dx_update::semantics::may_be_rewritten` stays false for both requirement
shapes); this command owns the single-requirement rewrite
(`dx_bump::BumpRequest::may_be_rewritten` is true), including exact pins,
bounded ranges, and Git tag/commit shapes per the ecosystem mapping in
`dx_bump::sets`. An explicit operation keeps the contract honest: no
`--widen` flag silently breaks the load-bearing invariant.

All ecosystems in v1, no phasing, covering the native seven-set scope:
Bazel modules plus `.bazelversion`, Cargo, npm/pnpm (both lock
graphs), Go (`gomod`), GitHub Actions, Maven, NuGet (issue #637).
Selector syntax is `set:package`
(`bazel:rules_rust`, `bazel:.bazelversion`, `cargo:anyhow`, `npm:react`,
`go:example.com/mod`, `github-actions:actions/checkout`,
`maven:junit:junit` plus `maven:org.junit.jupiter:junit-jupiter-api`,
`nuget:FSharp.Core` with `gha`/`gomod`
aliases canonicalized); bare sets, labels, paths, and empty versions fail
closed as usage errors (exit `2`), never as partial widens.

Library-first (ADR 0008): registry discovery, version comparison, and manifest
parsing use upstream libraries (BCR / crates.io / npm / Go proxy / Maven
Central / NuGet / GitHub releases clients plus `semver`, `serde_json`,
`toml`, `toml_edit`), never
custom HTTP/version/resolver code. Cargo edits preserve comments,
whitespace, and order through `toml_edit::DocumentMut`; `package.json`
stays on `serde_json::Value`. Custom code is limited to the thin
widen-one-requirement edit in `dx_bump::request`, loop orchestration, and PR
handling. All deps pin exactly per ADR 0008 (latest stable). Version shapes
validate through upstream `semver` (`dx_bump::version`): exact semver for
Bazel/Cargo/npm/Go/Maven/NuGet, tag or 40/64-char SHA for GitHub Actions (tags auto-resolve
to SHA via the upstream GitHub releases client before the file edit, planned in
`dx_bump::gha`, issue #640, fixtures in `cli/bump/tests/fixtures/bump_gha/`; manual SHA
only rejected, unknown tags fail closed with never an invented SHA).
Discovery proposes stable versions only; prerelease eligibility follows the
upstream resolver and project configuration
(`dx_bump::version::prerelease_follows_upstream`), never a private policy.
Transitives stay resolver-governed (`dx_update::semantics`
pins intact); bump never forces every transitive to newest.

Manifests widened atomically (one file per invocation): `.bazelversion` or
`MODULE.bazel` (Bazel, file-only), `rust/tests/fixtures/hello/Cargo.toml` (Cargo),
`package.json` (npm), `third_party/go/go.mod` (Go), `.github/workflows/ci.yml`
(GitHub Actions, SHA-plus-tag pins), `MODULE.bazel` `maven.install`
artifacts (Maven, e.g. `"junit:junit:4.13.2"`),
`third_party/dotnet/paket.dependencies` (NuGet, e.g. `nuget FSharp.Core 10.1.201`).
Lock refresh chains automatically and resolver-owned (issue #638, fixtures in
`cli/bump/tests/fixtures/bump_chain/`) for Cargo/npm/Go/Maven/NuGet
(`dx_bump::BumpRequest::refresh_selector`): `dx update cargo` (full; Cargo
selective is wont-fix), `dx update npm:<pkg>` (selective for the widened
package), `dx update go` (noop; pinned module lock tracks Gazelle, no
launch), `dx update maven` (whole-lock `REPIN=1` pin),
`dx update nuget` (whole-folder `paket2bazel` regen); Bazel and GitHub Actions
verify file-only through `preset.update --verify-only` flag-diff review plus
`bazel build //...`. Per-set selective support is decided in
[ADR 0024](../../decisions/0024-selective-update.md). Missing, ambiguous, or unsupported manifest shapes fail
closed with nothing widened (exit `1`, `bump_failed`). Refresh failures keep
the widen with no rollback and exit `1` with `update_failed`.

Loop (one dep per PR, never batch): discover outdated (stable only,
prerelease follows upstream, transitives stay resolver-governed) via the
upstream registry clients (BCR / crates.io / npm registry / Go proxy /
Maven Central / NuGet / GitHub releases, never custom HTTP; planned in
`dx_bump::discovery`, issue #639, fixtures in
`cli/bump/tests/fixtures/bump_discovery/`) → auto-resolve GitHub Actions tags
to SHA via the upstream GitHub releases client (`gh api`, never custom HTTP;
planned in `dx_bump::gha`, issue #640, fixtures in
`cli/bump/tests/fixtures/bump_gha/`) → widen
one requirement via `dx bump` (which chains its refresh automatically) →
run the bump-PR verification (regen evidence,
`preset.update --verify-only` flag-diff, `bazel build //...`,
`bazel test //...`, coverage/dogfood gates per
[local workflows](../../contributing/local-workflows.md#preset-update-loop))
→ if green open one PR, if red discard and record → reset to clean tree →
next dep. One dep per PR with an automerge on/off toggle only: when on,
auto-merge solely on the full required-check set; when off, leave PRs open.
No grouping, schedule, or dashboard knobs. Runner is the scheduled
`bump.yml` workflow with `GITHUB_TOKEN`, concurrency control so N open PRs do
not stampede CI (`concurrency: group: bump-widen-one-${{ github.ref }}`,
`cancel-in-progress: false`: runs queue, never cancel); scheduled runs without inputs enumerate outdated via the
upstream clients above (manual selector only rejected, issue #639);
failures never retry-until-green; fork-safety and the human
merge path from the [automation policy](../../contributing/automation.md)
preserved (fork PRs plan only, never push or open PRs with write
credentials; bot PRs are reviewed and merged by hand, never pushed to
`main`).

Runner contract (`bump.yml`, the only runner): CLI vs runner tree policy
is explicit. `dx bump` itself never checks a dirty tree: it rewrites the
working copy directly (no refusal, no clean requirement); the caller owns
tree state. The runner starts from a clean checkout and, after every dep
(`always()` step), discards via `git reset --hard HEAD` plus `git clean
-fd` plus `checkout main`, so the next dep starts clean: dirty state is
discarded, never refused. Red runs record in workflow logs plus the job
summary (`bump widen-one — <status>` in `$GITHUB_STEP_SUMMARY`, retained
as run evidence); there is no separate record file. Evidence retention is
the verification output in run logs (regen, flag-diff, `bazel build
//...`, `bazel test //...` with CI flags, coverage seed gate, dogfood).
Automerge input shape is the `workflow_dispatch` boolean `automerge`
(default `false`): `true` runs `gh pr merge --auto --squash` on the full
required-check set only; `false` leaves the PR open for human merge.

`dx bump` is mutating without confirmation like `dx update`. `--dry-run`
plans the widen plus the automatic refresh and exits `0` without touching
the tree or launching; live execution rewrites exactly one requirement
atomically then chains the refresh. JSON supports the shared
`command_started`/`command_finished` frame with widen `notice` plus refresh
`notice`/`error` events. Usage errors exit `2` pre-exec; widen failures exit
`1` with `bump_failed`; refresh failures exit `1` with `update_failed`.
`--check`, `--fail-on`, `--report`, `--output=diff`, and
`-- <bazel-options>` do not apply on this path.
