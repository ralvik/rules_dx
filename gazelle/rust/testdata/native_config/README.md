# Native-Config Generation

Verifies frozen filename recognition, config-target selection, scoped
visibility, and aspect_hints binding for tool-owned native configs.

Matrix: `full` (fresh sources plus rustfmt/vale configs with a
styles closure; clippy is unmanaged since #47), `merge` (hand target
wins, generated-name hint resolves to it), `removal` (deleted config
file stubs the generated target and clears the hint), `config_only`
(taplo target with no Rust sources), `skip` (directive subset omits
the skipped tool).
