import { add } from "./sums.js";

export function total(xs: number[]): number {
	return xs.reduce((acc, x) => add(acc, x), 0);
}
