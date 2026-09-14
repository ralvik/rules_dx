# Devcontainer

Implementation status: devcontainer scaffolding delivered through `dx init`
(`.devcontainer/devcontainer.json` with pinned base image, Bazel-delegated
`postCreateCommand`, no ambient tools).

The devcontainer definition is emitted through the same
Bazel-owned environment projections as `dx env` and `dx setup`, not as a
hand-maintained parallel toolchain. Container builds and non-Linux runs remain
gaps; no working container or multi-platform support is claimed until qualified
execution lands.
