#define WRITE_TO(addr, value) (*((volatile unsigned int*)(addr)) = value)
#define RAM_START 0x20000000

int main() {
    int a = 5;
    int b = 8;

    if (a < b) {
        WRITE_TO(RAM_START, 42);
    } else {
        WRITE_TO(RAM_START, 1);
    }

    if (a > b) {
        WRITE_TO(RAM_START + 4, 42);
    } else {
        WRITE_TO(RAM_START + 4, 1);
    }

    // Loop forever
    while (1) {}

    return 0;
}
