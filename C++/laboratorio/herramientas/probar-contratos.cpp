// SPDX-License-Identifier: GPL-2.0-only
#include "../../produccion/gpus.hpp"
#include "../../produccion/limites.hpp"
#include <array>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>

namespace {
void require(bool ok, const char* message) { if (!ok) throw std::runtime_error(message); }
}
int main() {
    try {
        constexpr auto max = std::numeric_limits<std::size_t>::max();
        constexpr std::array<std::int64_t, 8> dims{-1, 0, 1, 2, 8, 16384, 2147483647,
            std::numeric_limits<std::int64_t>::max()};
        constexpr std::array<std::size_t, 7> sizes{0, 1, 4, 63, 64, 1048576, max};
        std::size_t tested = 0;
        // Independent 128-bit oracle for 64-bit target arithmetic. No huge allocation.
        static_assert(sizeof(std::size_t) == 8);
        __extension__ using Wide = unsigned __int128;
        for (auto w : dims) for (auto h : dims) for (auto capacity : sizes) for (auto budget : sizes) {
            const auto result = ultra::cpu::rgba8(w, h, capacity, budget, std::numeric_limits<std::int64_t>::max());
            const Wide expected = w > 0 && h > 0 ? static_cast<Wide>(w) * static_cast<Wide>(h) * 4 : 0;
            const bool valid = w > 0 && h > 0 && expected <= max && expected <= capacity && expected <= budget;
            require(result.has_value() == valid, "layout diverge del oraculo");
            if (result) require(result->bytes == expected, "layout bytes");
            ++tested;
        }
        require(!ultra::cpu::rgba8(9, 1, 64, 64, 8), "limite dispositivo ignorado");
        require(!ultra::cpu::rgba8(1, 1, 4, 4, 0), "limite invalido aceptado");
        for (auto capacity : sizes) for (auto offset : sizes) for (auto length : sizes) {
            const bool expected = static_cast<Wide>(offset) + length <= capacity;
            require(ultra::cpu::range(capacity, offset, length) == expected, "range diverge del oraculo");
            ++tested;
        }
        std::cout << "VERDE [CPU-01]: " << tested << " contratos contra oraculo de 128 bits y dos limites\n";

        const std::array<std::uint8_t, 5> payload{0, 1, 2, 254, 255};
        auto good = ultra::gpus::encode(7, "identity", payload);
        require(good.has_value(), "encode valido");
        for (std::size_t size = 0; size < good->size(); ++size)
            require(!ultra::gpus::parse(std::span{*good}.first(size), "identity"), "prefijo truncado aceptado");
        for (std::size_t offset = 0; offset < good->size(); ++offset) {
            auto bad = *good; bad[offset] ^= 1;
            require(!ultra::gpus::parse(bad, "identity"), "mutacion de bit aceptada");
        }
        require(!ultra::gpus::encode(7, "", payload), "identidad vacia encode");
        require(!ultra::gpus::encode(7, "id", {}), "payload vacio encode");
        std::string large(ultra::gpus::max_key + 1, 'k');
        require(!ultra::gpus::encode(7, large, payload), "identidad excesiva encode");
        std::vector<std::uint8_t> oversized(ultra::gpus::max_binary + 1);
        require(!ultra::gpus::encode(7, "id", oversized), "payload excesivo encode");
        large.resize(ultra::gpus::max_key);
        oversized.resize(ultra::gpus::max_binary);
        auto boundary = ultra::gpus::encode(7, large, oversized);
        require(boundary.has_value() && ultra::gpus::parse(*boundary, large).has_value(), "limites validos rechazados");
        std::size_t structured = 0;
        for (std::size_t field : {std::size_t{0}, std::size_t{4}, std::size_t{8}, std::size_t{12}, std::size_t{16}}) {
            for (std::uint32_t edge : std::array<std::uint32_t, 6>{0, 1, 65536, 1048576, 0x7FFFFFFFU, 0xFFFFFFFFU}) {
                auto data = *good;
                ultra::gpus::write32(data, field, edge);
                ultra::gpus::write32(data, 20, ultra::gpus::checksum(data));
                const auto parsed = ultra::gpus::parse(data, "identity");
                if (parsed) {
                    const auto canonical = ultra::gpus::encode(parsed->format, "identity", parsed->binary);
                    require(canonical.has_value() && *canonical == data, "entrada estructurada aceptada no es canonica");
                } else {
                    require(field != 8, "parser pretende validar formatos de un driver que no conoce");
                }
                ++structured;
            }
        }
        auto forged = *good;
        forged.back() ^= 1;
        ultra::gpus::write32(forged, 20, ultra::gpus::checksum(forged));
        require(ultra::gpus::parse(forged, "identity").has_value(), "checksum se presenta incorrectamente como autenticacion");
        std::cout << "VERDE [CPU-02]: prefijos, mutaciones, limites maximos y rechazos del serializador\n";
        std::cout << "VERDE [CPU-04]: " << structured << " cabeceras con checksum valido; checksum no autentica contenido\n";

        std::uint32_t random = 0x31415926U;
        for (unsigned sample = 0; sample < 4096; ++sample) {
            std::vector<std::uint8_t> data(sample % 256);
            for (auto& byte : data) {
                random = random * 1664525U + 1013904223U;
                byte = static_cast<std::uint8_t>(random >> 24);
            }
            const auto result = ultra::gpus::parse(data, "identity");
            require(!result, "datos aleatorios inesperadamente validos");
        }
        std::cout << "VERDE [CPU-03]: 4096 entradas pseudoaleatorias acotadas rechazadas\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "ROJO: " << error.what() << '\n'; return 1;
    }
}
