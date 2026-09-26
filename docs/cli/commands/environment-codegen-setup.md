# Environment, Codegen, And Setup Commands

```sh
bazel run //dx:env
bazel run //cli/cli:dx -- setup
bazel run //cli/cli:dx -- env
bazel run //cli/cli:dx -- codegen
```

`dx setup` prepares codegen plus env in one go. `dx env` refreshes tools. `dx codegen` selects generated sources. With a label, each acts on that target only.
