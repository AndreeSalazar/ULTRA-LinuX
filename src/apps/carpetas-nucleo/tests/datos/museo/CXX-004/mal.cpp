#include <thread>
volatile bool listo = false;
void f() {
    std::thread t([] { listo = true; });
    while (!listo) {}
    t.join();
}
