#include <string>
#include <string_view>
void f() {
    std::string dueno = std::string("hola") + "!";
    std::string_view v = dueno;
    (void)v.size();
}
