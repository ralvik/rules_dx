# M29: Release Publication

## Outcome

The exact M28-qualified v1 module and CLI bytes are published to the approved destinations using
approved credentials, then verified through their public installation paths.

## Scope

Use the approved [distribution destinations](../environments/environment.md#distribution) and
execute only O45-approved credentialed operations for M28-qualified
bytes and documentation; create the release/tag and module registry entry, verify existing checksums,
signatures/provenance and notices, and smoke-test every advertised installation path from public locations.
Make the M28-qualified reusable consumer CI workflow/reporting revision and caller documentation
available through the approved release, then verify the documented public consumer setup path.

## Contract References

- [Dependency currency](../decisions/0008-dependency-currency.md), [tested platforms](../decisions/0014-tested-platform-release-stack.md), [tools](../tools/), [environments](../environments/), [testing](../testing/), and open decision [O45](../open-decisions.md).
- [Consumer GitHub CI contract](../github-ci.md) and [CI evidence matrix](../testing/github-ci.md).

## Deliverables

- Published module release, host CLI artifacts, checksums, provenance, notices, changelog, and release announcement.
- Release documentation links to the [maintenance policy](../environments/environment.md#distribution)
  so consumers can identify which release line receives fixes.
- Post-publication clean-consumer and standalone installation verification.
- Publicly consumable qualified reusable CI revision and caller/setup documentation, with clean-consumer
  verification through the documented release pins.

## Work Packages

1. Verify publication inputs, destinations, and credential scopes exactly match M28-qualified identities
   and the resolved publication policy.
2. Publish immutable release metadata, module, qualified CLI bytes, checksums, provenance, and notices.
3. Test public Bazel module and standalone installation on every advertised host.
4. Record publication locations and any incident without rebuilding or substituting bytes silently.
5. Verify public consumer CI pins resolve to the M28-qualified workflow/reporting identities and run
   a clean-consumer smoke test through the published caller/setup path without local substitutes.

## Milestone-Specific Evidence

- Public downloads match qualified digests and install without ambient Rust; the Bazel-first path yields the module-matched CLI.
- Registry/module resolution and every advertised host smoke test pass against public artifacts
  obtained through the approved distribution destinations, not local substitutes.
- The [private vulnerability reporting channel](../../SECURITY.md) is enabled and verified
  before public release, and the security policy accurately describes its availability.
- Public reusable-workflow/reporting identities match the M28 handoff; the documented caller runs
  selected checks and required reporting with explicit platform selection and truthful aggregate status.

## Out Of Scope

- Rebuilding or qualifying candidates, selecting installer/signing/provenance/publication policy,
  adding scope, post-v1 features, and promoting unqualified support cells.

## Completion Report Additions

- Record version/tag, approved destinations and credentialed operations, public URLs, immutable digests,
  signatures/provenance, registry state, smoke results, and publication incidents.
- Record public consumer CI pins and their qualified identities, caller/setup smoke evidence, and
  any publication mismatch without substituting an unqualified workflow or reporter.
