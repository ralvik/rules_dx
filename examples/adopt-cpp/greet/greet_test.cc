// Foreign C++ test: handwritten owner of the test-owned source. The
// generator never emits cc_test; `*_test.cc` files stay out of the
// generated library and this rule survives regeneration unchanged.
#include <cassert>
#include <iostream>

#include "examples/adopt-cpp/greet/greet.h"
#include "examples/adopt-cpp/greet/helper.h"

int main() {
  assert(Greet("dx") == "hello dx world");
  assert(HelperSuffix() == " world");
  std::cout << Greet("dx") << std::endl;
  return 0;
}
