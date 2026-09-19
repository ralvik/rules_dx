import { badge } from "./widgets.js";

test("badges a name", () => {
	expect(badge("poly")).toBe("[poly]");
});
