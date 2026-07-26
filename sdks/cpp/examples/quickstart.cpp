#include <cstdlib>
#include <iostream>

#include "swissarmynoife/sak_client.hpp"

int main() {
  const char* env = std::getenv("SAK_HTTP");
  swissarmynoife::SakClient sak(env ? env : "http://127.0.0.1:8787");
  std::cout << "health=" << sak.health().dump() << "\n";
  std::cout << "modules=" << sak.list_modules().dump() << "\n";
  return 0;
}
