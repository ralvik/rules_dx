import suffix from "./helper.js";

export function check() {
  if (suffix("x") !== "<x>") {
    throw new Error("suffix mismatch");
  }
}
