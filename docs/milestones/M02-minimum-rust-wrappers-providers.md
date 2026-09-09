# M02: Minimum Rust Wrappers And Providers

## Outcome

Minimal Rust wrappers expose the source and execution facts needed for the first quality slice.

## Scope

Add the narrow pinned Rust ruleset integration, wrappers for one library/binary/test path, and provider
normalization for direct checked-in Rust sources and selected toolchain components.

## Contract References

- [Rust foundation decision](../decisions/0013-rust-javascript-typescript-foundations.md), [toolchain versions](../decisions/0012-language-toolchain-versions.md), [quality contracts](../quality/), [testing](../testing/), and open decisions [O15 and O16](../open-decisions.md).

## Deliverables

- Experimental minimal Rust wrappers and provider adapters.
- External-consumer fixtures exposing Rust source classes and authoritative rustfmt/Clippy identities.

## Work Packages

1. Prove pinned upstream public providers and host/target/execution configuration behavior.
2. Add minimal wrappers while preserving upstream providers.
3. Normalize only the facts required by quality and later generation work.

## Milestone-Specific Evidence

- Fixtures prove one source owner, provider preservation, and no ambient Cargo/Rust tool discovery.
- Unused Rust integration performs no operational work beyond declared module metadata.

## Out Of Scope

- Complete Cargo target semantics, Gazelle generation, IDE environments, and public support.

## Completion Report Additions

- Record upstream symbols used, provider mappings, wrapper gaps, and toolchain-selection limits.
