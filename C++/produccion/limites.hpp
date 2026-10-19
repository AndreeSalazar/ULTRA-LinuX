// SPDX-License-Identifier: GPL-2.0-only
#pragma once
#include <cstddef>
#include <cstdint>
#include <expected>
#include <limits>

namespace ultra::cpu {
enum class Error { dimensions, limit, overflow, capacity, budget };
struct Rgba8 { std::size_t bytes; };
// Tightly packed, positive RGBA8. No GL calls, allocation or pointer access.
[[nodiscard]] constexpr std::expected<Rgba8, Error>
rgba8(std::int64_t width, std::int64_t height, std::size_t capacity,
      std::size_t budget, std::int64_t max_dimension) {
    if (width <= 0 || height <= 0 || max_dimension <= 0) return std::unexpected(Error::dimensions);
    if (width > max_dimension || height > max_dimension) return std::unexpected(Error::limit);
    const auto w = static_cast<std::uint64_t>(width);
    const auto h = static_cast<std::uint64_t>(height);
    constexpr auto max = std::numeric_limits<std::size_t>::max();
    if (w > max / 4) return std::unexpected(Error::overflow);
    const auto row = static_cast<std::size_t>(w) * 4;
    if (h > max / row) return std::unexpected(Error::overflow);
    const auto bytes = row * static_cast<std::size_t>(h);
    if (bytes > capacity) return std::unexpected(Error::capacity);
    if (bytes > budget) return std::unexpected(Error::budget);
    return Rgba8{bytes};
}
[[nodiscard]] constexpr bool range(std::size_t capacity, std::size_t offset, std::size_t length) {
    return offset <= capacity && length <= capacity - offset;
}
}
