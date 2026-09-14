// Test-owned fixture sources never enter the generated library:
// handwritten cc_test owns *_test.cc, including its main.
#include <cassert>

#include "pkg/demo/helper.h"

int main() {
  assert(HelperSuffix() == " world");
  return 0;
}
