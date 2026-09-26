# `dx generate`

```sh
bazel run //cli/cli:dx -- generate //...
bazel run //cli/cli:dx -- generate --check //...
```

Refreshes Gazelle `BUILD` files. No scope means the whole repo. `--check` fails if files are stale instead of writing them.
