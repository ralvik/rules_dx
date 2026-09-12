import suffix from "./helper.jsx";

export function check() {
  if (suffix("x") !== "<x>") {
    throw new Error("suffix mismatch");
  }
}
