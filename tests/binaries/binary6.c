#include "stdint.h"

int main() {
    asm("lw a4, 1(sp)");

    // Loop forever
    while (1) {}

    return 0;
}
