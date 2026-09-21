# Deploy

Contracts for `dx deploy` dispatch and the owner-gated release path:

- [Deploy Authoring](authoring.md): how targets become deployable
  (`dx_deployment`, `DxDeployInfo`, `archive_deploy`, `github_deploy`)
  and how standalone install verification works.
- [Release Runbook](release-runbook.md): owner-gated human-run release
  steps, dry-run first with signing before draft.
- [Offline Bootstrap](offline-bootstrap.md): vendored launcher plus
  advisory mirror bundle for airgapped hosts, checksum-verified with
  no download on the offline path.
