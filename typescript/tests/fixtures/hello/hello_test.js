// Seed Jest test over the tsc-compiled output. Static-import coverage
// of TypeScript sources needs the transform slice (see
// //javascript/tests/fixtures/hello:hello_test); until then the test imports the compiled
// `./hello.js` from the runfiles tree via `data = [":hello_lib"]`.
import { hello } from "./hello.js";

test("greets by name", () => {
	expect(hello("world")).toBe("hello world");
});
