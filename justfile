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

# Compiles the specified `.c` file in the `tests/binaries` directory
[working-directory: 'system_code']
@binary-compile filename:
    ../xpacks/.bin/riscv-none-elf-gcc -march=rv32i -T link.ld -nostdlib bootloader.S ../tests/binaries/{{filename}}.c -o ../tests/binaries/{{filename}}.raw
    ../xpacks/.bin/riscv-none-elf-objcopy -O binary -j .text ../tests/binaries/{{filename}}.raw ../tests/binaries/{{filename}}.bin

alias bc := binary-compile

# Shows disassembly of the specified `.c` file in the `tests/binaries` directory
[working-directory: 'system_code']
@binary-dump filename: (binary-compile filename)
    ../xpacks/.bin/riscv-none-elf-objdump -d ../tests/binaries/{{filename}}.raw -M no-aliases,numeric

alias bd := binary-dump
