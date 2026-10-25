#include <string>
#include <vector>
std::size_t total(std::vector<std::string> nombres, std::string separador) {
    return nombres.size() * separador.size();
}
