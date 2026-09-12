import helper from "./helper.jsx";
import fs from "fs";

export function greet(name) {
  return "hello " + name + helper(fs.sep);
}
