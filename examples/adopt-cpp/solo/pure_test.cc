// Foreign C++ solo test: handwritten owner of the test-owned source.
#include <cassert>

#include "examples/adopt-cpp/solo/pure.h"

int main() {
  assert(Add(2, 3) == 5);
  assert(Add(-1, 1) == 0);
  return 0;
}
