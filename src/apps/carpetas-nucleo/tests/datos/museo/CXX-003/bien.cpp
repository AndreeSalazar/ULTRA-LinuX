#include <memory>
struct T { ~T(); };
void f() {
    T *p = new T[4];
    delete[] p;
    auto q = std::make_unique<T[]>(4);
}
