# `dx init` And `dx hooks`

```sh
bazel run //cli/cli:dx -- init
bazel run //cli/cli:dx -- hooks install
```

`dx init` scaffolds a new repo. It never overwrites existing files.

`dx hooks install` installs `pre-commit` and `pre-push` shims. `dx hooks uninstall` removes them. `dx hooks status` shows what would run. `dx hooks run <trigger>` runs one trigger.
