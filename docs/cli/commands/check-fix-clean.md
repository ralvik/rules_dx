# `dx check`, `dx fix`, And `dx clean`

```sh
bazel run //cli/cli:dx -- check //...
bazel run //cli/cli:dx -- fix //...
bazel run //cli/cli:dx -- clean
```

## `dx check` And `dx fix`

Runs `format`, then `lint`, then `typecheck`, then `generate`. Stops on the first failure.

`dx check` only reports. `dx fix` applies fixes. Run `dx check` again after `dx fix` to confirm.

## `dx clean`

Prunes old `.dx` generations. `dx clean --bazel` also runs `bazel clean`. `dx clean --dry-run` only lists what would go.
