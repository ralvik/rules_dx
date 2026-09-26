# Mixed hello example

Mixed-framework composition fixture: Vue, Svelte, Astro, and MDX containers
over one shared JavaScript helper. Each container keeps its own wrapper plus
the shared core `helper_lib`; no container is owned twice and
framework-to-framework imports are absent by construction. The owners are
handwritten and generation never owns this tree. It is the framework-track
fixture, not an external-consumer workspace.

```sh
bazel build //examples/mixed/hello/...
bazel test //examples/mixed/hello/...
```

Evidence: the build covers the four framework libraries plus `helper_lib`
and `hello_test`. `hello_test` proves the shared helper edge, the
per-container helper references, and no framework-to-framework imports.

Scope notes: this tree is handwritten with no generated targets, so there is
no `dx init` or `dx generate` step and no lockfile scope. Cross-framework
imports resolve only through explicit Bazel deps to the shared helper.
