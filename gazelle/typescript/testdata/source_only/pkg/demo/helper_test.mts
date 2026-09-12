import suffix from "./helper";

export function check(): void {
  if (suffix("x") !== "<x>") {
    throw new Error("suffix mismatch");
  }
}
