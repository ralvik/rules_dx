# Local Workflows

## Current Workflow

Build, test, and coverage run through Bazel on Linux x86_64 with local-only
execution:

```sh
bazel build //...
bazel test //...
```

Coverage follows the mandatory project gate described in
[Testing](../testing/README.md#coverage):

```sh
bazel run //dx/cli:dx -- coverage --min-coverage <percent> //...
```

## Corpus Dogfood

The repository corpus (`real_source_target(name = "corpus")` per package)
is checked with the
real lint/format aspects; every produced result must pass the per-result
evaluator at `--fail_on warning`. The same invocations run in CI
(`.github/workflows/ci.yml`, job `corpus-dogfood`), which installs no
quality tools: all tools execute as Bazel-resolved pinned actions.

Select the corpus targets, then build their `dx_results`:

```sh
bazel query "attr(name, '^corpus$', kind(real_source_target, //...))" \
  | LC_ALL=C sort -u > /tmp/corpus_targets.txt
bazel build $(tr '\n' ' ' < /tmp/corpus_targets.txt) \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results
```

Pass the targets to `aquery` as a `+` union (a bare query expression over
`//...` widens the analyzed set): list the produced result protos, confirm
every corpus target owns results, and evaluate each one. Paths to
`--result` must be absolute: `bazel run` does not execute from the
workspace root.

```sh
union="$(paste -sd+ /tmp/corpus_targets.txt)"
bazel aquery "$union" \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text 2>/dev/null \
  | grep -o 'bazel-out/[^]]*\.pb' | sort -u \
  | sed 's|bazel-out/[^/]*/bin/|bazel-bin/|' > /tmp/corpus_results.txt
while read -r result; do
  bazel run //quality/evaluator:quality_evaluator -- \
    --result "$PWD/$result" --fail_on warning \
    --output "/tmp/corpus_eval_$(echo "$result" | tr '/' '_').marker"
done < /tmp/corpus_results.txt
```

Audit corpus ownership: every applicable file
(`BUILD.bazel`, `MODULE.bazel`, `*.bzl`, `*.toml`, `*.md`, excluding
generated locks and the `paket2bazel` hub output under
`third_party/dotnet/deps/`) must be owned by a corpus target. The reverse direction
is informational: siblings, native configs, and fixture-owned files are
corpus-referenced without being applicable sources.

```sh
head -1 third_party/dotnet/deps/paket.main.bzl | grep -q GENERATED
head -1 third_party/dotnet/deps/paket.main_extension.bzl | grep -qi GENERATED
git ls-files \
  | grep -E '(^|/)(BUILD\.bazel|MODULE\.bazel)$|\.(bzl|toml|md)$' \
  | grep -v -E '\.lock$' \
  | grep -v -E '^third_party/dotnet/deps/paket\.main(_extension)?\.bzl$' \
  | LC_ALL=C sort -u > /tmp/corpus_applicable.txt
bazel query "kind('source file', deps(kind(real_source_target, //...)))" 2>/dev/null \
  | grep -E '^(@@)?//' \
  | sed 's/^@@//; s|^//||; s|:|/|; s|^/||' \
  | LC_ALL=C sort -u > /tmp/corpus_closure.txt
comm -23 /tmp/corpus_applicable.txt /tmp/corpus_closure.txt
```

The last command prints nothing when every applicable file has a corpus
owner.

## Preset Update Loop

Shared Bazel execution flags live in the vendored preset (`tools/bazelrc`),
version-matched to `.bazelversion`. Review and change flags only through
the inventory in `tools/bazelrc/preset.py`:

```sh
bazel run //tools/bazelrc:preset.update -- --verify-only
bazel run //tools/bazelrc:preset.update
```

The verify command rejects stale generated files, prints the flag diff
under review, and rejects root `.bazelrc` lines that duplicate preset
flags (reconcile by removing the owned duplicates; project overrides stay
explicit and `user.bazelrc` stays last). `preset.update_test` pins the pin
and the inventory in `bazel test //...`. Owned build profiles
(`dx_debug`/`dx_dev`/`dx_release`) are reviewed the same way; see
[ADR 0021](../decisions/0021-build-profiles.md).

Version bumps arrive as Renovate PRs (`renovate.json`, `bazel` manager, no
auto-merge). The loop stays manual: run the regen, review the flag diff,
update the test pins, run full verification (`bazel build //...`,
`bazel test //...`, plus the corpus dogfood above), then merge by hand.

## Linux-First Bring-Up

Bring-up starts Linux-first with local-only execution. Record unavailable
required hosts from
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms)
as gaps at entry; do not claim them. Linux-first is bring-up order, not a scope
reduction: v1 retains full required-host coverage.

## Local-Only Coverage

Coverage is local-only. The mandatory
[project coverage gate](../testing/README.md#coverage) must pass;
Codecov service activation and fork-PR handling are tracked in
[GitHub coverage reporting](../testing/README.md#github-coverage-reporting).
Do not present local reports as service evidence.

## Local Overrides

`dx init` absent-only scaffolding and `dx hooks install`/`status`/`run` dispatch are
implemented as specified in the [`dx init` and `dx hooks` contract](../cli/commands/hooks.md).
The gitignored root overlay `dx.local.toml` with its `[hooks]` table is created absent-only
by `dx hooks install`; the committed typed `hooks` workspace-policy section stays the team
baseline and CI never reads the personal overlay. This checkout does not consume the overlay
beyond hook-shim merging. Exact schema stays under the
[hooks contract](../cli/commands/hooks.md); this is not a current setup step beyond hooks.
