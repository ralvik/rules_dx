# M28: Stabilization And Release Qualification

## Outcome

The v1 APIs, command surface, support matrix, artifacts, and compatibility policy are frozen and release-qualified.

## Scope

Resolve provisional release blockers, freeze public load labels and protocols, complete docs/examples/notices/module metadata,
resolve installer/signing and artifact publication-policy qualifications, run the full repository and
consumer matrices, and build reproducible candidate module and CLI artifacts.
Requalify the existing repository coverage gate against the complete release implementation.
Requalify M27's reusable consumer GitHub CI against the release revision; its exact integration
mappings must already be frozen before affected M27 implementation, not first selected here.
M28 consumes the release-blocking docs subset (M30a); remaining adoption scope stays post-release (M30b)
under O54.

## Contract References

- [Decisions](../decisions/), [architecture](../architecture/), [CLI](../cli/), [generation](../generation/), [quality](../quality/), [tools](../tools/), [environments](../environments/), [testing](../testing/), and open decisions [O6, O37, O38, and O39](../open-decisions.md).
- [Mandatory coverage policy](../testing/README.md#coverage) and the measurement mechanics resolved
  there (O47 resolved); release qualification does not defer that resolution.
- [Consumer GitHub CI contract](../github-ci.md) and [CI evidence matrix](../testing/github-ci.md).
- [M30a release-blocking docs](M30-adoption-bootstrap-first-hour.md#m30a-release-blocking-docs), a recorded M28 dependency.

## Deliverables

- Frozen public APIs/protocols, compatibility tests, complete command registry, support matrix, release notes, and candidate artifacts.
- Final generated tested-stack, tool, license, notice, and provenance metadata.
- Qualified standalone installer/signing format and artifact host, trusted-builder, SBOM, provenance,
  reproducibility, and publication policy inputs for M29.
- Release-revision line-coverage reports, ignore/reason validation, and reconciled implementation
  inventories, with any permitted Starlark behavioral fallback reported separately.
- Release-qualified consumer CI workflow/reporting identities, caller template, and setup documentation
  tied to external-consumer evidence and the publication handoff to M29.

## Work Packages

1. Resolve release-blocking provisional decisions, including O38 and O39 qualification choices, from
   accumulated evidence.
2. Freeze APIs, schemas, load labels, compatibility, tested stack, and support definitions.
3. Complete user documentation, examples, changelog, notices, and module metadata.
4. Run full platform, consumer, parity, generation, environment, cache, remote, laziness, and release suites.
5. Build and independently verify reproducible release candidates and every artifact that M29 may publish.
6. Reconcile every first-party implementation language and generated authored-logic path with the
   resolved coverage policy, and rerun the release coverage gate without blanket exclusions.
7. Run the consumer CI matrix against the candidate workflow/reporting revision and module-matched
   CLI; verify documented caller pins, required settings, platform selections, event/revision and
   merge-queue behavior, fork security, reporting lifecycle, and aggregate failure semantics.

## Milestone-Specific Evidence

- Every `Supported` cell has required-platform and external-consumer evidence; partial cells remain explicitly non-supported.
- Every required cell in the reviewed v1 inventory is qualified. A partial or non-supported label
  does not permit shipping an unresolved required capability on any required platform.
  Additional-foundation deferrals need the [admission-policy decision](../product/scope.md#first-release-admission);
  they waive neither required-core obligations nor the unchanged quality-tool baseline.
- The final registry contains exactly the accepted commands; qualified artifacts reproduce from a clean
  release environment and satisfy the resolved O38/O39 signing, provenance, and publication policy.
- [Artifact verification fixtures](../testing/tools.md#research-qualification-fixtures) prove embedded
  constituent provenance and detached attestations bound to the exact final archive bytes.
- Compatibility fixtures and release notes classify public API changes under the accepted
  [release-versioning policy](../environments/environment.md#distribution), separately from
  native tool-output changes and protocol schema compatibility.
- The central coverage gate passes for the release revision, including additional implementation
  languages and the required Starlark evidence. Release-revision negative fixtures reject missing reports,
  invalid or unreasoned ignores, and uncovered non-ignored executable lines rather than grant release
  waivers. Any Starlark behavioral fallback retains documented instrumentation infeasibility under the
  pinned real Bazel and meaningful assertion evidence under the [central policy](../testing/README.md#coverage).
- Clean external-consumer CI evidence identifies the exact candidate workflow, reporter, caller,
  module/CLI, and runner/platform mappings; required CI matrix gaps block release qualification.

## Out Of Scope

- Publishing tags, registries, releases, or artifacts; new v1 scope; and unmeasured API stabilization.

## Completion Report Additions

- Record final APIs, support matrix, resolved O38/O39 qualification choices, candidate digests,
  reproducibility, signing/provenance verification, notices, and release blockers.
- Separate evidence-backed additional-foundation deferrals from required-core blockers
  and other unresolved required cells; confirm all admitted required cells qualify and quality-tool
  obligations remain intact.
- Link exact coverage counts, instrumentation and aggregation identities, source classifications,
  uncovered-source diagnostics, ignore/reason validation, and missing-report failure evidence to the
  qualified revision; link any Starlark fallback's infeasibility and inventory/assertion evidence,
  keeping behavioral completeness distinct from instrumented line coverage.
- Link consumer CI matrix results and qualified workflow/reporting/caller identities to the release
  revision and record the exact publication and public-consumer verification inputs for M29.
