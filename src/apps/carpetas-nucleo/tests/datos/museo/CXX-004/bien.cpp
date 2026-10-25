#include <atomic>
#include <thread>
std::atomic<bool> listo{false};
void f() {
    std::thread t([] { listo.store(true); });
    while (!listo.load()) {}
    t.join();
}
