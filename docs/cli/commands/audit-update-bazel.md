# Audit, Update, And Bazel Commands

## `dx bazel`

```text
dx bazel <bazel arguments...>
```

Runs Bazel directly. Args after `bazel` go to Bazel unchanged. Put `dx` flags before `bazel`.

## `dx security` And `dx license`

```text
dx security [scope...]
dx license [scope...]
```

`dx security` checks for secrets and known bad deps. `dx license` checks dep licenses. No scope means `//...`. Use `--here` for the current dir.

```sh
bazel run //cli/cli:dx -- security //...
bazel run //cli/cli:dx -- license //...
```

## `dx update`

```text
dx update [--check] [set...]
```

Updates deps. `dx update --check` fails if the preset is stale. No scope updates all sets.

```sh
bazel run //cli/cli:dx -- update --check
bazel run //cli/cli:dx -- update go
```

## `dx bump`

```text
dx bump <set:package> <version>
```

Widens one dep requirement. Then run `dx update <set>`.

```sh
bazel run //cli/cli:dx -- bump go:example 1.2.3
```
