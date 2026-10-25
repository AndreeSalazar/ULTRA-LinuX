int suma(int a, int b) { return a + b; }
void f(void) {
    ((void (*)(double))suma)(1.0);
}
