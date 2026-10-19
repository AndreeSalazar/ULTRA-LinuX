// SPDX-License-Identifier: GPL-2.0-only
// Preflight CPU: does not open a display, socket, device or GPU context.
#include <expected>
#include <format>
#include <iostream>
#include <limits>
#include <string_view>

[[nodiscard]] constexpr std::expected<std::size_t, std::string_view>
checked_product(std::size_t count, std::size_t element_size) {
    if (element_size != 0 && count > std::numeric_limits<std::size_t>::max() / element_size)
        return std::unexpected("overflow");
    return count * element_size;
}

int main() {
    constexpr auto good = checked_product(1920, 4);
    constexpr auto bad = checked_product(std::numeric_limits<std::size_t>::max(), 4);
    static_assert(good.has_value() && *good == 7680);
    static_assert(!bad.has_value());
    static_assert(checked_product(0, 4).value() == 0);
    std::cout << std::format("CXX: gcc={} cplusplus={} libstdcxx={}\n",
                             __VERSION__, __cplusplus, __GLIBCXX__);
    std::cout << "CONTROL: expected, format y limites aritmeticos disponibles\n"
                 "ALCANCE: sonda CPU; no demuestra soporte completo de C++26 ni GL/EGL\n";
}
