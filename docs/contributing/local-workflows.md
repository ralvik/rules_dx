# Local Workflows

## Current Workflow

Build, test, and coverage run through Bazel on Linux x86_64 with local-only
execution:

```sh
bazel build //...
bazel test //...
```

Coverage follows the mandatory project gate described in
[Testing](../testing/README.md#coverage): `bazel coverage //...` plus
`bazel run //tools/coverage:check`.

## Corpus Dogfood

The repository corpus (`real_source_target(name = "corpus")` per package,
see [M05](../milestones/M05-direct-bazel-dogfood.md)) is checked with the
real lint/format aspects; every produced result must pass the per-result
evaluator at `--fail_on warning`. The same invocations run in CI
(`.github/workflows/ci.yml`, job `corpus-dogfood`), which installs no
quality tools: all tools execute as Bazel-resolved pinned actions.

Select the corpus targets (currently 22), then build their `dx_results`:

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
generated locks) must be owned by a corpus target. The reverse direction
is informational: siblings, native configs, and fixture-owned files are
corpus-referenced without being applicable sources.

```sh
git ls-files \
  | grep -E '(^|/)(BUILD\.bazel|MODULE\.bazel)$|\.(bzl|toml|md)$' \
  | grep -v -E '\.lock$' \
  | LC_ALL=C sort -u > /tmp/corpus_applicable.txt
bazel query "kind('source file', deps(kind(real_source_target, //...)))" 2>/dev/null \
  | grep -E '^(@@)?//' \
  | sed 's/^@@//; s|^//||; s|:|/|; s|^/||' \
  | LC_ALL=C sort -u > /tmp/corpus_closure.txt
comm -23 /tmp/corpus_applicable.txt /tmp/corpus_closure.txt
```

The last command prints nothing when every applicable file has a corpus
owner.

## Planned Linux-First Bring-Up

M00 starts Linux-first with local-only execution per
[M00](../milestones/M00-bazel-rust-ci-seed-quality.md). Record unavailable
required hosts from
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms)
as gaps at entry; do not claim them. Linux-first is bring-up order, not a scope
reduction: v1 retains full required-host coverage.

## Planned Local-Only Coverage

M00 coverage is local-only. The mandatory
[project coverage gate](../testing/README.md#coverage) must pass at M00 exit;
Codecov service activation and fork-PR handling remain under the
[O14 qualification requirements](../testing/README.md#github-coverage-reporting).
Do not present local reports as service evidence.

## Planned Local Overrides

Future `dx init`/`dx hooks install` setup is planned to create a root
`dx.local.toml` overlay with a `[hooks]` table and add its gitignore entry.
The planned overlay merges per-person over the committed typed `hooks`
workspace-policy section, with both triggers configurable in both layers.
This checkout does not consume the overlay or currently gitignore it.
Exact schema stays under [O49](../open-decisions.md) and the
[hooks contract](../cli/commands/hooks.md); this is not a current setup step.
