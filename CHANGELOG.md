# Changelog

All notable changes are recorded here. Status vocabulary: no `Supported`
cells are claimed; platform/product claims follow the support matrix.

No release has been cut. Tags, GitHub releases, registry submissions, and
`dist/`/`release/` outputs require explicit owner approval before they are
created.

## Unreleased

* Current tree: `dx` CLI, language foundations (Rust, Python,
  JavaScript/TypeScript, Vue, Svelte, Astro, MDX, plus admitted additional
  foundations), quality workflows, generation, environments/codegen/setup,
  audit/update, consumer CI contract, and adoption surfaces (`init`, `hooks`,
  `status`, `version`, `docs`, `watch`, `owners`/`deps`/`why`, `completion`).
* Evidence: `bazel build //...`, `bazel test //...`, and
  `bazel run //dx:generate_check` on the seed host. Non-Linux platforms, external-consumer runs, and trusted-builder
  provenance are not claimed here.
* Delivery record: milestone specs plus completion reports, the implementation
  plan, and the frozen `Oxx` decision register were removed during cleanup
  and live only in git
  history before that cleanup (see `git log --all --oneline` for the `Mxx`/`Oxx`
  entries). Per [CONTRIBUTING.md](CONTRIBUTING.md) and
  [docs/AGENTS.md](docs/AGENTS.md), no new `Mxx` specs or `Oxx` register entries
  are created; see the [roadmap](docs/roadmap.md).
