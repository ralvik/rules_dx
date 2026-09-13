// Package mdx implements the first-party MDX Gazelle extension
// (O42, M21). It is implemented directly against Gazelle and the
// selected public JavaScript ruleset APIs over the pinned MDX
// document boundary. There is no generic container parser
// or shared helper layer: helpers are extracted only after a later
// framework implementation proves concrete reuse.
package mdx
