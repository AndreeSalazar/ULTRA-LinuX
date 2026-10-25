#include <vector>
void f(std::vector<int> &v) {
    std::size_t i = 0;
    v.push_back(7);
    int &primero = v[i];
    primero = 3;
}
