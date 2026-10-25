struct T { ~T(); };
void f() {
    T *p = new T[4];
    delete p;
}
