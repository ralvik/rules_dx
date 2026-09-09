# Devcontainer Preview

Provisional M30 direction only. This preview creates no support claim and
changes no milestone scope.

M30 owns devcontainer support alongside onboarding guides, `dx init`
scaffolding, the custom hermetic hook runner, the consolidated diagnostics
surface, and `dx` versioning with rollback. See
[M30](../milestones/M30-adoption-bootstrap-first-hour.md) and the
[milestone index](../milestones/README.md).

The devcontainer definition will be generated or checked in through the same
Bazel-owned environment projections as `dx env` and `dx setup`, not as a
hand-maintained parallel toolchain. Exact image, feature, and mount mappings
freeze with the M30 work packages.
