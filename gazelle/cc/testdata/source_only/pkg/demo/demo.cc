// Source-only generation fixture: one directory, one generated
// cc_library, toolchain-provided angle includes only alongside one
// self-owned quoted include (no edge leaves the package).
#include <string>

#include "pkg/demo/helper.h"

// Greet returns a greeting for name.
std::string Greet(const std::string& name) {
  return "hello " + name + HelperSuffix();
}
