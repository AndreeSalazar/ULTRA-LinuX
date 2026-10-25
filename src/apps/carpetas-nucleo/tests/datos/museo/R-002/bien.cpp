#include <string>
#include <string_view>
#include <vector>
std::size_t total(const std::vector<std::string>& nombres, std::string_view separador) {
    return nombres.size() * separador.size();
}
