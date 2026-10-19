// SPDX-License-Identifier: GPL-2.0-only
#pragma once
#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <expected>
#include <span>
#include <string_view>
#include <vector>

namespace ultra::gpus {
// Experimental GLES container, little endian. Checksum detects damage, not forgery.
inline constexpr std::size_t header_size = 24;
inline constexpr std::size_t max_key = 65536;
inline constexpr std::size_t max_binary = 1048576;
enum class Error { truncated, magic, version, limits, length, checksum, identity };
enum class Mode { static_only, dynamic_rebuild };
struct View { std::uint32_t format; std::span<const std::uint8_t> binary; };
inline std::uint32_t read32(std::span<const std::uint8_t> data, std::size_t offset) {
    std::uint32_t value = 0;
    for (unsigned i = 0; i < 4; ++i) value |= std::uint32_t{data[offset + i]} << (i * 8);
    return value;
}
inline void write32(std::span<std::uint8_t> data, std::size_t offset, std::uint32_t value) {
    for (unsigned i = 0; i < 4; ++i) data[offset + i] = static_cast<std::uint8_t>(value >> (i * 8));
}
inline std::uint32_t checksum(std::span<const std::uint8_t> data) {
    std::uint32_t value = 2166136261U;
    for (std::size_t i = 0; i < data.size(); ++i) {
        if (i >= 20 && i < 24) continue;
        value = (value ^ data[i]) * 16777619U;
    }
    return value;
}
[[nodiscard]] inline std::expected<View, Error>
parse(std::span<const std::uint8_t> data, std::string_view expected_key) {
    if (data.size() < header_size) return std::unexpected(Error::truncated);
    if (read32(data, 0) != 0x53555047U) return std::unexpected(Error::magic);
    if (read32(data, 4) != 1) return std::unexpected(Error::version);
    const auto key_size = read32(data, 12);
    const auto binary_size = read32(data, 16);
    if (key_size == 0 || key_size > max_key || binary_size == 0 || binary_size > max_binary)
        return std::unexpected(Error::limits);
    const std::size_t total = header_size + std::size_t{key_size} + std::size_t{binary_size};
    if (data.size() != total) return std::unexpected(Error::length);
    if (read32(data, 20) != checksum(data)) return std::unexpected(Error::checksum);
    const std::string_view key{reinterpret_cast<const char*>(data.data() + header_size), key_size};
    if (key != expected_key) return std::unexpected(Error::identity);
    return View{read32(data, 8), data.subspan(header_size + key_size, binary_size)};
}
[[nodiscard]] inline std::expected<std::vector<std::uint8_t>, Error>
encode(std::uint32_t format, std::string_view key, std::span<const std::uint8_t> binary) {
    if (key.empty() || key.size() > max_key || binary.empty() || binary.size() > max_binary)
        return std::unexpected(Error::limits);
    std::vector<std::uint8_t> data(header_size + key.size() + binary.size());
    write32(data, 0, 0x53555047U);
    write32(data, 4, 1);
    write32(data, 8, format);
    write32(data, 12, static_cast<std::uint32_t>(key.size()));
    write32(data, 16, static_cast<std::uint32_t>(binary.size()));
    std::copy(key.begin(), key.end(), reinterpret_cast<char*>(data.data() + header_size));
    std::copy(binary.begin(), binary.end(), data.begin() + static_cast<std::ptrdiff_t>(header_size + key.size()));
    write32(data, 20, checksum(data));
    return data;
}
}
