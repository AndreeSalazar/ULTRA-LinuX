unsigned vendedor(void) {
    unsigned a;
    __asm__ volatile("cpuid" : "=a"(a) : "a"(0));
    return a;
}
