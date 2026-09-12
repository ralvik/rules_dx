import { hello } from "./hello.js";

// M16 seed Jest test. The root package.json sets `"type": "module"` so
// first-party `.js` sources are ESM everywhere (node, jest); the
// `node_options = ["--experimental-vm-modules"]` on the target enables
// jest's ESM support. Static-import coverage of TypeScript sources
// needs the transform slice (a later M16 slice wires babel-jest).
test("greets by name", () => {
  expect(hello("world")).toBe("hello world");
});
