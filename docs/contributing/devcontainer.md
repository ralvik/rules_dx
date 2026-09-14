# Devcontainer

Implementation status: devcontainer scaffolding delivered in M30b through `dx init`
(`.devcontainer/devcontainer.json` with pinned base image, Bazel-delegated
`postCreateCommand`, no ambient tools).

M30 owns devcontainer support alongside onboarding guides, `dx init`
scaffolding, the custom hermetic hook runner, the consolidated diagnostics
surface, and `dx` versioning with rollback. See
[M30](../milestones/M30-adoption-bootstrap-first-hour.md) and the
[milestone index](../milestones/README.md).

The devcontainer definition is emitted through the same
Bazel-owned environment projections as `dx env` and `dx setup`, not as a
hand-maintained parallel toolchain. Container builds and non-Linux runs remain
gaps; no working container or multi-platform support is claimed until qualified
execution lands.
