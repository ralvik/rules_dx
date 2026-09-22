# Environments

Developer environment and generated-source projection contracts:

- [Developer Environments](environment.md): PATH bootstrap, language-neutral selection, managed tools.
- [Managed Environment State](managed-state.md): immutable generations, setup selection, reuse, concurrency, ownership, retention.
- [Generated Code](codegen.md): generated-source execution and editor projection.
- [Rust Environment](rust.md): Rust toolchain, source, and rust-analyzer projection.
- [Node Environment](node.md): pnpm importer and managed `node_modules` projection.
- [Python Environment](python-environment.md): later concrete `.venv` projection.
- [Go Environment](go.md): focused `go_env_plan` over wrapper direct sources.
- [Java Environment](java.md): focused `java_env_plan` over wrapper direct sources.
- [Kotlin Environment](kotlin.md): focused `kotlin_env_plan` over wrapper direct sources.
- [Scala Environment](scala.md): focused `scala_env_plan` over wrapper direct sources.
- [C# Environment](csharp.md): focused `csharp_env_plan` over wrapper direct sources.
- [F# Environment](fsharp.md): focused `fsharp_env_plan` over wrapper direct sources.
- [C/C++ Environment](cc.md): focused `cc_env_plan` over wrapper direct sources.
- [PowerShell Environment](powershell.md): focused `powershell_env_plan` over wrapper direct sources.
- [Ruby Environment](ruby.md): focused `ruby_env_plan` over wrapper direct sources.
- [Vue Environment](vue.md): focused `vue_env_plan` over `JsInfo` plus direct sources.
- [Svelte Environment](svelte.md): focused `svelte_env_plan` over `JsInfo` plus direct sources.
- [Astro Environment](astro.md): focused `astro_env_plan` over `JsInfo` plus direct sources.
- [MDX Environment](mdx.md): focused `mdx_env_plan` over `JsInfo` plus direct sources.

Delivery order is dogfood-first: PATH/environment bootstrap precedes the Rust foundation, Python follows later.


## Qualification

Provider-derived plan mapping and per-foundation evidence lives in [Foundation Qualification](foundation-qualification.md).

```sh
bazel run //cli/cli:dx -- env //...
```
