// SPDX-License-Identifier: GPL-2.0-only
#pragma once
#include <algorithm>
#include <array>
#include <chrono>
#include <cstdio>
#include <time.h>
#include <sys/resource.h>

namespace ultra::lab {
inline bool measurement_allowed() {
#ifdef ULTRA_CXX_DIAGNOSTICO
    std::puts("ROJO [MEDICION]: compilacion instrumentada rechazada");
    return false;
#else
    return true;
#endif
}

// Wall time includes completion/readback and correctness checks, not just API dispatch.
// RSS is the cumulative process high-water mark, not memory allocated by one route.
template<class Workload>
bool compare(const char* api, Workload workload) {
    if (!workload(false) || !workload(true)) return false; // Unrecorded warmup of both routes.
    std::array<std::array<long long, 7>, 2> times{};
    for (unsigned round = 0; round < 7; ++round) {
        for (unsigned position = 0; position < 2; ++position) {
            const unsigned route = (round + position) % 2;
            timespec cpu_start{}, cpu_finish{};
            if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &cpu_start) != 0) return false;
            const auto start = std::chrono::steady_clock::now();
            const bool valid = workload(route != 0);
            const auto finish = std::chrono::steady_clock::now();
            if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &cpu_finish) != 0) return false;
            rusage usage{};
            if (!valid || getrusage(RUSAGE_SELF, &usage) != 0) return false;
            const auto elapsed = static_cast<long long>(
                std::chrono::duration_cast<std::chrono::nanoseconds>(finish - start).count());
            if (elapsed <= 0) return false;
            times[route][round] = elapsed;
            const auto cpu_ns = static_cast<long long>(cpu_finish.tv_sec - cpu_start.tv_sec) * 1000000000LL +
                               static_cast<long long>(cpu_finish.tv_nsec - cpu_start.tv_nsec);
            if (cpu_ns < 0) return false;
            const double cpu_us = static_cast<double>(cpu_ns) / 1000.0;
            std::printf("BENCH,%s,%u,%s,%lld,%.3f,%ld\n", api, round,
                route == 0 ? "recrear" : "reutilizar", elapsed, cpu_us, usage.ru_maxrss);
        }
    }
    for (auto& route : times) std::sort(route.begin(), route.end());
    std::printf("BENCH_RESUMEN,%s,recrear,min_ns=%lld,mediana_ns=%lld,max_ns=%lld\n",
        api, times[0].front(), times[0][3], times[0].back());
    std::printf("BENCH_RESUMEN,%s,reutilizar,min_ns=%lld,mediana_ns=%lld,max_ns=%lld\n",
        api, times[1].front(), times[1][3], times[1].back());
    std::puts("VERDE [MEDICION]: 7 pares con orden alternado y resultado comprobado");
    return true;
}
}
