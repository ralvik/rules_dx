import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { helper } from "./helper.js";

const here = path.dirname(fileURLToPath(import.meta.url));

describe("mixed/hello", () => {
	test("shared helper resolves the single cross-framework edge", () => {
		expect(helper("world")).toBe("hello world");
	});

	test("each framework container references the shared helper", () => {
		for (const file of [
			"Hello.vue",
			"Hello.svelte",
			"Hello.astro",
			"Hello.mdx",
		]) {
			const source = fs.readFileSync(path.join(here, file), "utf8");
			expect(source).toMatch("./helper.js");
		}
	});

	test("no framework container imports another framework container", () => {
		const containers = [
			"Hello.vue",
			"Hello.svelte",
			"Hello.astro",
			"Hello.mdx",
		];
		for (const file of containers) {
			const source = fs.readFileSync(path.join(here, file), "utf8");
			for (const other of containers) {
				if (other !== file) {
					expect(source).not.toMatch(other);
				}
			}
		}
	});
});
