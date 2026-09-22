// Entry fixture: a `main.cc` basename without a `main` definition is an
// ordinary library source, never a thin binary.
#include <string>

#include "pkg/demo/helper.h"

// EntryGreet returns a greeting for name.
std::string EntryGreet(const std::string& name) {
  return "entry hello " + name + HelperSuffix();
}
