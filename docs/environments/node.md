# Node Environment

JavaScript and TypeScript contribute provider-derived plans to the private environment collection
described by [Developer Environments](environment.md). Shared identity, installation, selection, and
retention follow [Managed Environment State](managed-state.md).

## Importer Projection

Each plan uses exact pnpm importer and package-store artifacts selected by the pinned
`aspect_rules_js` integration. It never invokes `pnpm install`, performs a second resolution, scans
output trees, or reconstructs lockfile semantics.

Gazelle maintains `npm_link_all_packages()` and its generated `:node_modules` target for each
importer. The adapter consumes declared outputs and fixture-proven public `JsInfo` or `DefaultInfo`
metadata. Package bytes remain shared in the upstream store while each represented importer keeps a
conventional managed `node_modules` facade at its workspace-relative path.

A repository-default plan includes every registered importer. A focused target plan includes every
complete importer represented by that target's configured closure, including its selected
development and type dependencies. Exact selection does not derive a smaller package tree from
individual import edges.

## Selection Behavior

Importer additions, removals, renames, lifecycle/build artifacts, peer-version variants, and logical
path conflicts follow authoritative provider output. Unrepresented importers are absent from a first
focused selection; later focused refreshes carry forward unrelated selected importer views through
the shared managed-state contract.

The public `dx env` repository/root/exact-target orchestration and atomic selection are delivered by
the repository-workflow milestone. The language foundation proves focused plans and projections
without exposing a public persistent-language provider.
