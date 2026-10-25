void g(int, int);
void f() {
    int i = 0;
    int a = i++;
    int b = i++;
    g(a, b);
}
