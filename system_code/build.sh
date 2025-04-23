#!/usr/bin/zsh

riscv32-none-elf-gcc -T link.ld -nostdlib bootloader.S main.c -o main
riscv32-none-elf-objcopy -O binary -j .text main main.bin
