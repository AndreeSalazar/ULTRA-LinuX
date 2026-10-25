typedef int (*Op)(int, int);
int suma(int a, int b) { return a + b; }
void f(void) {
    Op op = suma;
    op(1, 2);
}
