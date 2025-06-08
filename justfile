# List all available commands
default:
  @just --list -u

# Set up the local development environment
bootstrap:
    xpm install
    rustup show
    cargo clean
    rustup component add clippy
    cargo install cargo-binstall --locked
    cargo binstall cargo-nextest -y
    cargo build
    @echo "✅ Bootstrap complete ✅"

# Run all tests
test:
  cargo nextest run --no-fail-fast

# Runs benchmarks
bench:
  cargo bench

# Cleans all binary artifacts
binary-clean:
    rm -rf ./system_code/v1/build
    rm -rf ./system_code/v2/build

# Compiles the specified `.c` file in the `tests/binaries` directory, version 1
[working-directory: 'system_code/v1']
@binary-compile-1 filename:
    mkdir -p build
    rm -f build/binary*
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -T link.ld -nostdlib crt0.S ../../tests/binaries/{{filename}}.c -o build/{{filename}}.elf
    ../../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .text build/{{filename}}.elf ../../tests/binaries/{{filename}}.bin

# Compiles the specified `.c` file in the `tests/binaries` directory, version 2
[working-directory: 'system_code/v2']
@binary-compile-2 filename:
    mkdir -p build
    rm -f build/binary*
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -I inc -c -ffreestanding -nostdlib src/boot.c -o build/boot.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -I inc -c -ffreestanding -nostdlib src/crt0.S -o build/crt0.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -I inc -c -ffreestanding -nostdlib src/isr.c -o build/isr.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -I inc -c -ffreestanding -nostdlib ../../tests/binaries/{{filename}}.c -o build/{{filename}}.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -T link.ld -ffreestanding -nostdlib build/*.o -o build/{{filename}}.elf
    ../../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .text build/{{filename}}.elf build/{{filename}}.code
    ../../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .data build/{{filename}}.elf build/{{filename}}.data
    cat build/{{filename}}.code build/{{filename}}.data > ../../tests/binaries/{{filename}}.bin

# Compiles the specified `.c` file in the `tests/binaries` directory, version 3
[working-directory: 'system_code/v3']
@binary-compile-3 filename:
    mkdir -p build
    rm -f build/binary*
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i_zicsr -I inc -c -ffreestanding -nostdlib src/boot.c -o build/boot.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i_zicsr -I inc -c -ffreestanding -nostdlib src/crt0.S -o build/crt0.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i_zicsr -I inc -c -ffreestanding -nostdlib src/isr.c -o build/isr.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i_zicsr -I inc -c -ffreestanding -nostdlib ../../tests/binaries/{{filename}}.c -o build/{{filename}}.o
    ../../xpacks/.bin/riscv-none-elf-gcc -march=rv32i_zicsr -T link.ld -ffreestanding -nostdlib build/*.o -o build/{{filename}}.elf
    ../../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .text build/{{filename}}.elf build/{{filename}}.code
    ../../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .data build/{{filename}}.elf build/{{filename}}.data
    cat build/{{filename}}.code build/{{filename}}.data > ../../tests/binaries/{{filename}}.bin


# Shows disassembly of the specified `.c` file in the `tests/binaries` directory, version 1
[working-directory: 'system_code/v1']
@binary-dump-1 filename: (binary-compile-1 filename)
    ../../xpacks/.bin/riscv-none-elf-objdump -d build/{{filename}}.elf -M no-aliases,numeric

# Shows disassembly of the specified `.c` file in the `tests/binaries` directory, version 2
[working-directory: 'system_code/v2']
@binary-dump-2 filename: (binary-compile-2 filename)
    ../../xpacks/.bin/riscv-none-elf-objdump -d build/{{filename}}.elf -M no-aliases,numeric

# Shows disassembly of the specified `.c` file in the `tests/binaries` directory, version 2
[working-directory: 'system_code/v3']
@binary-dump-3 filename: (binary-compile-3 filename)
    ../../xpacks/.bin/riscv-none-elf-objdump -d build/{{filename}}.elf -M no-aliases,numeric

alias bc := binary-compile-3
alias bd := binary-dump-3
