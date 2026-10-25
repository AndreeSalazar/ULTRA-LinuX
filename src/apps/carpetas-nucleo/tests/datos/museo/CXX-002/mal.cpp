#include <vector>
void f(std::vector<int> &v) {
    int &primero = v[0];
    v.push_back(7);
    primero = 3;
}
