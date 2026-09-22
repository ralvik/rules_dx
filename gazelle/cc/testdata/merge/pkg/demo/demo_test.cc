// Test-owned merge fixture sources never enter the generated library.
#include <cassert>

#include "pkg/demo/helper.h"

int main() {
  assert(HelperSuffix() == " world");
  return 0;
}
