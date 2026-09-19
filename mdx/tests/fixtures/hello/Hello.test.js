import { compile } from "@mdx-js/mdx";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const source = fs.readFileSync(path.join(here, "Hello.mdx"), "utf8");

let program;
beforeAll(async () => {
	// `@mdx-js/mdx` exposes the documented asynchronous `compile` over
	// the pinned remark/micromark pipeline; `outputFormat: "program"`
	// yields the full compiled JS module whose top-level `import`
	// statements are exactly the document's ESM edges.
	program = String(await compile(source, { outputFormat: "program" }));
});

describe("Hello.mdx", () => {
	test("compiles the ESM import to exactly one first-party edge", () => {
		const importLines = program
			.split("\n")
			.filter((line) => line.startsWith("import "));
		const firstParty = importLines.filter((line) => line.includes("./"));
		expect(firstParty).toHaveLength(1);
		expect(firstParty[0]).toMatch("./helper.js");
	});

	test("keeps fenced-code imports out of the module edge set", () => {
		const importLines = program
			.split("\n")
			.filter((line) => line.startsWith("import "));
		expect(importLines.some((line) => line.includes("fake"))).toBe(false);
	});
});
