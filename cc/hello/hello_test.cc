// M22 seed C++ test; consumer of dx_cc_test.
#include <cassert>

#include "cc/hello/hello.h"

int main() {
  assert(Add(2, 3) == 5);
  assert(Add(-1, 1) == 0);
  return 0;
}
