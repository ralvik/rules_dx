# Changelog

All notable changes are recorded here. Platform and product status follows the
[support matrix](docs/product/support-matrix.md).

No release has been cut yet. Tags, GitHub releases, registry submissions, and
`dist/`/`release/` outputs require an explicit release gate before they are
created. Published bytes are never rebuilt or substituted silently.

## Unreleased

* Current tree: `dx` CLI, language foundations (Rust, Python,
  JavaScript/TypeScript, Vue, Svelte, Astro, MDX, plus admitted additional
  foundations), quality workflows, generation, environments/codegen/setup,
  audit/update, consumer CI contract, and adoption surfaces (`init`, `hooks`,
  `status`, `version`, `watch`, `owners`/`deps`/`why`, `completion`).
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
* Known issues: no cell is `Supported` yet; promotion needs platform plus
  consumer plus release evidence per the
  [support matrix](docs/product/support-matrix.md). Only the seed-host closure
  is claimed as evidence here; external-consumer runs and trusted-builder
  provenance remain owned gaps.
* Deprecated: the JUnit Vintage runner (JUnit 4 on the Platform) stays only as
  the deprecated seed path beside the Jupiter console-launcher shape (see
  `java/tests/fixtures/junit/BUILD.bazel`); ESLint stylistic core rules (for
  example `semi`, reported under `usedDeprecatedRules`) stay deprecated
  upstream as moved to `@stylistic` (see
  [tool integrations](docs/quality/tool-integrations.md)).
* Upcoming: planned work is tracked in the [roadmap](docs/roadmap.md); no new
  release outputs are claimed here until the release gate passes.
