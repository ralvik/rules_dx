# Devcontainer

Implementation status: the `dx init` devcontainer definition
(`.devcontainer/devcontainer.json` with pinned base image, Bazel-delegated
bootstrap `postCreateCommand`, no ambient tools) is dogfooded in this
repository. The checked-in definition is held byte-identical to the
scaffold output by `//.devcontainer:devcontainer_parity_test`, and CI runs
that gate check-only on every push and pull request. `postCreateCommand`
runs the user-path bootstrap (`bazel run //dx:env`, then `dx setup`)
instead of a full build, so container create pays only
managed-environment setup.

Container boot (devcontainer CLI build plus `postCreateCommand` execution
asserting the managed environment materializes) and non-Linux runs remain
open gaps; no working container or multi-platform support is claimed until
qualified execution lands.
