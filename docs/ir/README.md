# IR

Versioned Documentation IR schema and codec owned here; design
contracts live in the Documentation domain:

- [Schema](doc_ir.proto): `dx.documentation.v1` source of truth.
- [Codec](ir/src/lib.rs): `//docs/ir/ir:documentation_ir`
 validate, encode, and decode helpers.
- Documentation IR contract: authoritative
 model, validation, and drift policy.
- Site build: authoritative action design
 that consumes the generated shards.
