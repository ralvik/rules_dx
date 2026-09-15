# Changelog

All notable changes are recorded here. Status vocabulary: no `Supported`
cells are claimed; platform/product claims follow the support matrix.

No release has been cut. Tags, GitHub releases, registry submissions, and
`dist/`/`release/` outputs require explicit owner approval before they are
created; see [issue #5](https://github.com/ralvik/rules_dx/issues/5).

## Unreleased

* Current tree: `dx` CLI, language foundations (Rust, Python,
  JavaScript/TypeScript, Vue, Svelte, Astro, MDX, plus admitted additional
  foundations), quality workflows, generation, environments/codegen/setup,
  audit/update, consumer CI contract, and adoption surfaces (`init`, `hooks`,
  `status`, `version`, `docs`, `watch`, `owners`/`deps`/`why`, `completion`).
* Evidence: `bazel build //...`, `bazel test //...`, and
  `bazel run //dx:generate_check` on the seed host. Non-Linux platforms, external-consumer runs, and trusted-builder
  provenance are tracked in the issue tracker, not claimed here.
* Delivery record: milestone specs plus completion reports, the implementation
  plan, and the frozen `Oxx` decision register were removed under
  [#11](https://github.com/ralvik/rules_dx/issues/11) and live only in git
  history before that cleanup (see `git log --all --oneline` for the `Mxx`/`Oxx`
  entries). Per [CONTRIBUTING.md](CONTRIBUTING.md) and
  [docs/AGENTS.md](docs/AGENTS.md), no new `Mxx` specs or `Oxx` register entries
  are created; planned work lives in
  [GitHub issues](https://github.com/ralvik/rules_dx/issues) per
  [roadmap](docs/roadmap.md).
