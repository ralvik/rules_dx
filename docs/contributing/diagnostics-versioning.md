# Diagnostics And Versioning Preview

Provisional O50/O51 direction only; no `dx` implementation exists yet. This
preview creates no new API and changes no milestone scope.

There is no `dx doctor` command per
[ADR 0006](../decisions/0006-cli-command-surface.md). The planned requirement is
for every command to emit structured environment diagnostics in its machine
output; M30 is planned to add one consolidated status surface over the same
data. Exact owning command, status vocabulary, and machine-readable shape freeze under
[O50](../open-decisions.md). See the [hooks contract](../cli/commands/hooks.md)
for the two-layer hook configuration this surface reports on.

Planned per-repository `dx` version pinning uses a Bazelisk-style launcher
resolving a checked-in pin; rollback is re-pinning the previous release. Pin
format and update mechanics freeze under [O51](../open-decisions.md). See
[distribution](../environments/environment.md#distribution).
