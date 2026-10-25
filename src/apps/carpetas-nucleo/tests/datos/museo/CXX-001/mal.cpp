#include <string>
#include <string_view>
std::string nombre();
void f() {
    std::string_view v = std::string("hola") + "!";
    (void)v.size();
}
