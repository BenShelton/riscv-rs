#define WRITE_TO(addr, value) (*((volatile unsigned int*)(addr)) = value)
#define RAM_START 0x20000000

int main() {
    unsigned long result;
    asm("rdinstret %0" : "=r"(result));

    // Loop forever
    while (1) {}

    return 0;
}
