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

# Compiles the `main.c` file in the `system_code` directory
[working-directory: 'system_code']
@system-compile:
    ../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -T link.ld -nostdlib bootloader.S main.c -o main
    ../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .text main main.bin

# Shows the disassembly of the compiled `main.c` binary
[working-directory: 'system_code']
@system-dump: system-compile
    ../xpacks/.bin/riscv-none-elf-objdump -d main -M no-aliases,numeric
