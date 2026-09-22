// Merge fixture: one directory, one generated cc_library.
#include <string>

#include "pkg/demo/helper.h"

// Greet returns a greeting for name.
std::string Greet(const std::string& name) {
  return "hello " + name + HelperSuffix();
}
