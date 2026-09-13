// Helper implementation for the source-only generation fixture: the
// quoted self-include resolves to the owning library and leaves no edge.
#include "pkg/demo/helper.h"

#include <string>

std::string HelperSuffix() {
  return " world";
}
