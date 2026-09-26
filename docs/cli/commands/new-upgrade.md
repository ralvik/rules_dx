# `dx new` And `dx upgrade`

```sh
bazel run //cli/cli:dx -- new rust my_project
bazel run //cli/cli:dx -- upgrade --from 1.0.0 --to 2.0.0
```

`dx new <language> [name]` scaffolds a project. It never overwrites existing files.

`dx upgrade --from <v> --to <v>` runs pin, migrate, and setup in one go. No manifests exist yet, so live runs fail closed. `--dry-run` only prints the plan.
