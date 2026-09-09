# Environments

Developer environment and generated-source projection contracts:

- [Developer Environments](environment.md): PATH bootstrap, language-neutral selection, and managed
  tools.
- [Managed Environment State](managed-state.md): immutable generations, setup selection, reuse,
  concurrency, ownership, and retention.
- [Generated Code](codegen.md): generated-source execution and editor projection.
- [Rust Environment](rust.md): Rust toolchain, source, and rust-analyzer projection.
- [Node Environment](node.md): pnpm importer and managed `node_modules` projection.
- [Python Environment](python-environment.md): later concrete `.venv` projection.

The delivery order is dogfood-first: PATH/environment bootstrap precedes the Rust language
foundation, and Python follows as a later language projection.
