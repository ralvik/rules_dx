# Contributing

Start with the repository [contributor guide](../../CONTRIBUTING.md).
Product and design authority stays in the linked contracts; this directory
adds no new semantics.

- [Local workflows](local-workflows.md): current manual checks and tooling gaps;
  planned Linux-first bring-up, coverage, and local overrides.
- [Devcontainer](devcontainer.md): scaffolding through `dx init` (implemented in `cli/adopt`),
  not a commitment to working container support.
- [Diagnostics and versioning](diagnostics-versioning.md): implemented `dx status`/`dx version`
  surface and `dx` pinning.
- [Automation](automation.md): allowed bots and bot-opened PRs; native bump
  loop as sole updater with update-only automerge under guardrails.
- [BUILD conventions](build-conventions.md): one-line refs for repeated
  `BUILD.bazel` patterns.
