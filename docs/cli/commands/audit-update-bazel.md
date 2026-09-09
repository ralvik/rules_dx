# Audit, Update, And Bazel Commands

## `dx bazel`

```text
dx bazel <bazel arguments...>
```

Invokes the repository's Bazelisk-compatible `bazel` launcher with arguments
unchanged. The launcher reads the committed `.bazelversion`. `dx bazel` does not
perform scope resolution. This is the advanced-user escape hatch.

## `dx audit`

```text
dx audit [security|license] [scope ...] [--report <format>=<destination> ...]
```

Provisional implementation status: the audit policy below is accepted, but tool selection,
advisory acquisition/snapshot semantics, severity/report mappings, and native configuration
remain unqualified under [O11](../../open-decisions.md), update mappings under
[O12](../../open-decisions.md), and the license family under
[O58](../../open-decisions.md). Do not implement audit/update integrations against this prose
until those qualifications land.

Bare `dx audit` runs both families. `dx audit security` runs secrets plus
dependency-vulnerability analysis only; `dx audit license` runs license-policy
analysis only. Family selection composes with the normal scope resolution below;
it does not change scope defaults.

`dx audit` applies the non-mutating audit aspect and any approved ecosystem dependency-
audit integrations to the selected Bazel scope. It has two families, `security`
(secrets plus dependency-vulnerability analysis) and `license` (dependency
license-policy analysis); it is not an umbrella
for lint, formatting, tests, or builds. Gitleaks is the selected initial secrets integration,
acquired as a checksummed standalone artifact with SARIF output and secret-value redaction.
Research notes (unproven mappings): observed upstream `v8.30.1` with per-OS/arch archives plus
`--report-format json|csv|junit|sarif|template`, `--report-path`, `--redact` for logs/stdout,
TOML discovery (`--config`, `GITLEAKS_CONFIG`, `GITLEAKS_CONFIG_TOML`, `.gitleaks.toml`, else built-in
defaults), and conflated exit `1` for leaks or errors with `--exit-code` override. Report-file
redaction, findings-versus-operational-error distinction, and silent-`0` cases need fixtures; recheck
the latest stable and re-pin exact bytes at implementation. Trufflehog is a future depth option,
not v1 scope. The proposed
`secrets` policy-family mapping is its own semantic class with SARIF and secret-value redaction;
qualify that registry amendment against the single-family registry in
[Quality Sources](../../quality/quality-sources.md) before implementation.
The proposed `secrets` policy-family mapping
must be reconciled with source-class applicability under [O11](../../open-decisions.md).
Dependency-vulnerability tools, sources, exact ecosystem mappings, and acquisition/report
proofs remain unqualified; selecting Gitleaks does not establish working audit support.
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
This dependency-set scope does not broaden source-audit selection. Exact target-to-owner mappings
and upstream full-lock audit coverage require qualification under O11.

Dependency audits automatically refresh applicable vulnerability advisory data through supported
upstream tooling when invoked; a separate manual refresh is not the default workflow. Supply an
identified advisory snapshot as an input to Bazel-owned analysis, so results and cache identity
reflect the data actually analyzed rather than an untracked live database inside the audit action.
Acquisition/cache updates do not change application dependency versions, manifests, lockfiles, or
selected environment/codegen projections. Auditors remain pinned tools; advisory freshness does not
authorize automatic tool-version upgrades. If required advisory refresh fails, fail the audit and
report that current data could not be obtained. Do not fall back to a stale snapshot for the affected
dependency audit or report that dependency set as clean. Exact acquisition, snapshot identity, and
refresh/cache semantics remain open under O11.

Keep dependency inventories out of external vulnerability services. Download applicable advisory
databases and match packages against the identified snapshots within Bazel-owned analysis; do not
upload lockfiles or send dependency package names and versions through query parameters, request
bodies, or auditor telemetry. Package-specific advisory requests that disclose the inventory are
not an alternative to local matching. Qualify database-download and offline-matching routes under
O11; a query-only upstream service does not satisfy this contract. This restriction concerns
vulnerability services and does not change separately configured Bazel remote execution/cache
boundaries for declared analysis inputs.

Audit supports SARIF 2.1.0 reports through the shared `--report` contract. Partial
collection marks the SARIF invocation unsuccessful while retaining validated findings.

If a selected dependency cannot be assessed by the qualified auditor, fail audit as incomplete and
identify the dependency and assessment limitation. Examples include unsupported Git revisions or
unidentified private packages. Retain validated findings from assessed dependencies, but do not
claim complete coverage or treat an unassessed dependency as having no known vulnerabilities.
A recognized, assessable package with no matching advisories is a different result and is not
itself a coverage failure. Advisory-specific risk acceptance does not waive missing assessment.
Qualify upstream assessment evidence, dependency identities, and incomplete-report mappings under
O11; an empty findings list alone is not evidence that every selected dependency was assessed.

Report known vulnerabilities whether or not a fixed version is available, and apply the same
severity threshold and failure policy in both cases. Lack of a fix must not suppress a finding,
downgrade its severity, or exempt it from failure. Preserve upstream remediation information when
available, without treating a dependency-version upgrade as an automatic source fix or mutating
dependencies during audit. Exact advisory and report mappings require qualification under O11.

Known applicable vulnerabilities with no severity rating fail audit by default. Report the
upstream advisory severity as unknown text rather than inventing a rating or silently
treating missing metadata as non-blocking. The normalized diagnostic level stays in the
closed `info|warning|error` set owned by the [Output Protocol](../output-protocol.md#diagnostic).
A valid explicit risk-acceptance exception may exempt the finding from failure while retaining its
visibility. Qualify the distinction between upstream vulnerability severity and normalized diagnostic
level, threshold interaction, and text/structured/SARIF representation under O11 before implementation;
this decision does not introduce an unreviewed severity enum or report schema.

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
and must not present it as fixed. Qualify text, structured-output, and SARIF mappings under O11
before implementation; no new event fields or schema are selected here.
Exact native configuration, identity/alias matching, validation and report mappings, and exception
lifecycle mappings remain open under O11. Lack of a fix alone is not an implicit exception.

Every vulnerability risk-acceptance exception requires an expiration date. Missing, invalid, or
expired dates fail validation; an expired exception no longer exempts its finding from the normal
failure policy. Renewal requires an explicit reviewed configuration change, not automatic extension
by the auditor. Qualify date syntax, expiration boundary/time zone, evaluation-time input, native
configuration, and cache behavior under O11. An earlier cached acceptance must not allow a later
audit invocation to pass after expiry. This requirement applies to vulnerability risk acceptance,
not to the separate declared-dependency usage exceptions.

Obsolete risk exceptions fail validation even before expiration. An exception must still match an
applicable vulnerability in its complete owning dependency set against the current advisory
snapshot; removing the dependency or upgrading beyond the affected versions makes it obsolete.
Report the obsolete entry for explicit removal, without deleting it automatically. Do not infer
obsolescence from failed/incomplete analysis or from an unrelated owner being outside the selected
audit scope. Exact matching, advisory-alias handling, and validation mappings require O11 qualification.

Remaining audit integration details follow these established defaults: fail incomplete assessment,
keep matching local to declared advisory snapshots, preserve truthful visible findings, and permit
only narrow, explained, version-scoped, expiring risk acceptance. Prefer the simplest conforming
upstream integration. O11 owns technical qualification of tools, native configuration, identity and
severity mappings, date/time and cache semantics, and reporting. These details do not require more
product-preference decisions unless evidence reveals a contract conflict or requires new public API;
no implementation or verified audit support is approved by these policy choices alone.

### License family (`dx audit license`)

The outcome and exception rules below are accepted. Use qualified per-root (per-dependency) attribution over
conservative whole-lock strictness, without silently narrowing complete-lock audit coverage.
Full-lock tier attribution remains
unresolved when internal and distributed roots share a lock; no strictest-tier or per-root
attribution strategy is selected here. Resolve that boundary, per-ecosystem license-identity
mappings, approval/report mappings, and proof evidence under [O58](../../open-decisions.md)
before implementation.

The license family reuses security-audit scope mechanics (default `//...`,
per-target owning dependency sets, complete-lock coverage, local matching with
no lockfile upload) and exception lifecycle (version-scoped with upstream
version semantics, reasoned, expiring with ISO-8601 UTC dates evaluated at
audit time, obsolete only when no applicable finding remains, always visible). An upgrade
within an exception's bounded version range retains acceptance while the exception still
matches the finding and remains otherwise valid; upgrading alone does not invalidate it.
Its report format is SPDX 2.3
JSON via the shared `--report` contract: one
document per invocation, package IDs as package URLs, `DESCRIBES` relations
from each audited root, `CONTAINS` relations where the lock graph is known.

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
set = "cargo-lock"
license = "GPL-3.0-only"
versions = ">=1.2.0, <2.0.0"
reason = "Legal approved for internal fork; re-review on major bump."
expires = "2027-03-01"
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
(copyright notice plus text). Those words come from each package archive's
`LICENSE*`/`NOTICE*` files, delivered as declared Bazel inputs per package so
future NOTICE aggregation is hermetic and cached. A package whose license requires
reproduction but ships no text reports `missing-notice-text`, which fails in
`distributed` unless excepted. Collecting the texts now keeps the data ready;
assembling and bundling an aggregated NOTICE artifact into releases is out of
scope until the deferred packaging/publishing pipeline exists, at which point
it consumes these already-validated inputs.

Validation (all fail the audit, none auto-repair):

- Unknown label under `[distribution]` fails as `unknown_distribution_root`.
- A license ID in more than one policy list fails.
- An exception with no applicable finding fails as obsolete, like vuln
  exceptions; missing, invalid, or expired dates fail; out-of-range versions
  do not inherit acceptance.

## `dx update`

With no selection, `dx update` updates all supported dependency sets in the repository,
independent of the current working directory. It does not restrict updates to the containing
ecosystem workspace. Dependency-set discovery and selection use approved Bazel integrations,
not a CLI filesystem scan or a second dependency graph.

V1 supports selecting dependency sets (ecosystem workspaces/lockfiles) and individual packages
within selected sets. Delegate package selection to the upstream updater; necessary transitive
changes remain permitted under its resolver semantics. Package selection is not a guarantee that
only one lockfile entry changes, nor permission to silently substitute an update of the entire set
when the upstream integration cannot support the requested selection. Exact selector syntax and
ecosystem package-identity mappings remain subject to O12 qualification.

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
Exact ecosystem mappings, including moved-tag behavior, require qualification under O12.

Transitive dependencies remain governed by upstream resolution; the command does not force every
transitive package to its newest release regardless of compatibility. A newer release outside the
declared requirements is not an update failure. Resolution failures or unsupported update operations
must be reported rather than claimed as successful updates.
A successful dependency update is not a guarantee that application code still builds or passes tests.

Failure in one dependency set does not stop updates to independent selected sets. Preserve successful
changes, report each failure, and return an overall failure status if any selected update fails.
Do not run operations that depend on a failed update; report them as blocked, not successful.
This is not a repository-wide transaction or rollback. Independence must follow the approved
upstream integration: sets sharing a lockfile or resolver workspace cannot be treated as independent
merely because they have different Bazel labels.
This is the explicit update exception to [common fail-fast handling](../cli-contract.md#exit-status),
not permission to run dependents of failed operations or introduce a private scheduler.
Exact aggregate exit-code selection, backend operation boundaries, and per-set success/failure/blocked
reporting remain under O12. Continued updates do not imply parallel execution or a new mutation-event API.

Invoking `dx update` authorizes immediate application without an interactive confirmation prompt or
separate acceptance flag, in both terminal and noninteractive use. This does not bypass separate
license-consent or mutation-safety requirements.

The command introduces no `dx` lockfile or dependency resolver.
Every changed file and invoked operation must be attributable to the
underlying updater. Ordinary builds and editor activity do not initiate dependency-version upgrades.
Supported ecosystem mappings, selective-update syntax, remaining non-registry dependency handling, and exact
upstream operation/report mappings remain open under [O12](../../open-decisions.md).
