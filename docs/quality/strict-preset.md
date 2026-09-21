# Strict Preset

Opt-in strict preset decision (issue #615). Quality only.

## Decision

Accepted (issue #615): the default stays loose and strict is opt-in.

Default loose means omitted selections use curated defaults and enabled
tools use pinned upstream built-in defaults unless an applicable
checked-in native config supplies policy, per
[Native Configuration](native-configuration.md#authority) and the
[curated baseline](../tools/tool-baseline.md#curated-differences).
Upgrading `rules_dx` never tightens a consumer's lint: only the consumer
editing their own checked-in config does.

Strict opt-in means a consumer copies the strict native-config examples
below into their own package and reruns generation. Gazelle binds the
configs through `aspect_hints`; there is no workspace-level strict flag,
no selectable preset ID, and no hidden behavioral preset. Unselected
opt-ins create no actions or fetches, per the curated baseline.

Forcing strict on all consumers by default is rejected. It would break
upgrades and, per the
[default lifecycle](../tools/tool-baseline.md#default-lifecycle-direction),
would require a major release with compatibility qualification, release
notes, and a tested preservation override.

## Loose Defaults

Without an applicable native config, each tool runs its pinned upstream
built-in defaults with only transport and hermetic settings added by the
adapter. The repository's own loose pins are not consumer defaults: the
root `ruff.toml` (`E4`, `E7`, `E9`, `F`) and the empty `biome.json` are
this workspace's checked-in policy, proven to match upstream
config-free behavior by the no-config golden tests in
[Quality Workflow Testing](quality-testing.md#configured-policy).

## Strict Examples

Copy these files into the owning package; do not reference this
repository's paths. Each strict example is a strict superset of its
loose counterpart: anything the loose config flags, the strict config
also flags. Pinned fixtures live in
`quality/tests/fixtures/strict_preset/` via
`bazel run //tools/ci:strict_preset_qualification`.

Ruff strict (`ruff.toml`) selects stable rule families beyond the loose
`E4`, `E7`, `E9`, `F` set:

```toml
[lint]
select = ["E", "F", "W", "I", "N", "UP", "B", "SIM"]
```

Biome strict (`biome.json` only, `biome.jsonc` stays wont-fix per issue
#589) enables the recommended set plus strict errors. The pinned Biome
is 2.5.12; recheck the schema at upgrade:

```json
{
  "$schema": "https://biomejs.dev/schemas/2.5.12/schema.json",
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "correctness": {
        "noUnusedVariables": "error"
      },
      "style": {
        "useConst": "error"
      },
      "suspicious": {
        "noExplicitAny": "error"
      }
    }
  }
}
```

TypeScript strict (`tsconfig.json`) is `"strict": true`:

```json
{
  "compilerOptions": {
    "strict": true
  }
}
```

Vale strict stays markers-only (`Dx.Markers` with `MinAlertLevel =
suggestion`, same as the repository corpus). Prose rules stay wont-fix
per issues #589 and #665: native configuration owns policy with no hidden preset,
so there is no prose strict preset to opt into.

## Compatibility

Quality only. Existing consumers see no behavior change: curated
defaults, formatter sets, and upstream pins are unchanged, and the
strict examples are additive checked-in configs, not a default change.

## Gate

The gate is `bazel run //tools/ci:strict_preset_qualification`.
