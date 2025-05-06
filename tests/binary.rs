use riscv::{RV32ISystem, State, system_interface::MMIODevice};

macro_rules! run_instruction {
    ($rv:expr) => {
        $rv.cycle();
        $rv.cycle();
        $rv.cycle();
        $rv.cycle();
        $rv.cycle();
        assert_eq!($rv.state, State::Fetch);
    };
}

macro_rules! run_to_line {
    ($rv:expr, $line:expr) => {
        while $rv.current_line() != $line {
            run_instruction!($rv);
        }
    };
}

fn load_binary(filename: &str) -> Vec<u32> {
    let root_dir = std::env::current_dir().expect("Failed to get current directory");
    let binaries_dir = root_dir.join("tests/binaries");
    let binary_data =
        std::fs::read(binaries_dir.join(filename)).expect("Failed to read binary file");
    binary_data
        .chunks(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("Invalid chunk size")))
        .collect()
}

#[test]
fn test_binary_1() {
    let instructions = load_binary("binary1.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    // 10000000:    20400137    lui sp,0x20400
    run_instruction!(rv);
    assert_eq!(rv.reg_file[2], 0x2040_0000);

    // 10000004:    ffc10113    addi x2,x2,-4 # 203ffffc <_ebss+0x3ffffc>
    run_instruction!(rv);
    assert_eq!(rv.reg_file[2], 0x203F_FFFC);

    // 10000008:    03c0006f    jal	x0,10000044 <main>
    run_instruction!(rv);

    // 10000044:    fe010113    addi x2,x2,-32
    run_instruction!(rv);
    assert_eq!(rv.reg_file[2], 0x203F_FFDC);

    // 10000048:    00112e23    sw x1,28(x2)
    run_instruction!(rv);

    // 1000004c:    00812c23    sw x8,24(x2)
    run_instruction!(rv);

    // 10000050:    02010413    addi x8,x2,32
    run_instruction!(rv);
    assert_eq!(rv.reg_file[8], 0x203F_FFFC);

    // 10000054:    fb9ff0ef    jal x1,1000000c <fortyTwoWithSideEffects>
    run_instruction!(rv);
    assert_eq!(rv.reg_file[1], 0x1000_0058);

    // 1000000c:    ff010113    addi x2,x2,-16
    run_instruction!(rv);
    assert_eq!(rv.reg_file[2], 0x203F_FFCC);

    // 10000010:    00112623    sw x1,12(x2)
    run_instruction!(rv);

    // 10000014:    00812423    sw x8,8(x2)
    run_instruction!(rv);

    // 10000018:    01010413    addi x8,x2,16
    run_instruction!(rv);

    // 1000001c:    200007b7    lui x15,0x20000
    run_instruction!(rv);
    assert_eq!(rv.reg_file[15], 0x2000_0000);

    // 10000020:    30041737    lui x14,0x30041
    run_instruction!(rv);
    assert_eq!(rv.reg_file[14], 0x3004_1000);

    // 10000024:    f0070713    addi x14,x14,-256 # 30040f00 <_ebss+0x10040f00>
    run_instruction!(rv);
    assert_eq!(rv.reg_file[14], 0x3004_0F00);

    // 10000028:    00e7a023    sw x14,0(x15) # 20000000 <_ebss>
    run_instruction!(rv);
    assert_eq!(rv.bus.read_word(0x2000_0000), Ok(0x3004_0F00));

    // 1000002c:    02a00793    addi x15,x0,42
    run_instruction!(rv);
    assert_eq!(rv.reg_file[15], 0x0000_002A);

    // 10000030:    00078513    addi x10,x15,0
    run_instruction!(rv);
    assert_eq!(rv.reg_file[10], 0x0000_002A /* 42 */);

    // 10000034:    00c12083    lw x1,12(x2)
    run_instruction!(rv);

    // 10000038:    00812403    lw x8,8(x2)
    run_instruction!(rv);
    assert_eq!(rv.reg_file[8], 0x203F_FFFC);

    // 1000003c:    01010113    addi x2,x2,16
    run_instruction!(rv);

    // 10000040:    00008067    jalr x0,0(x1)
    run_instruction!(rv);

    // 10000058:    fea42623    sw x10,-20(x8)
    run_instruction!(rv);
    assert_eq!(rv.bus.read_word(0x203F_FFE8), Ok(0x0000_002A) /* 42 */);

    // 1000005c:    00200693    addi x13,x0,2
    run_instruction!(rv);
    assert_eq!(rv.reg_file[13], 0x0000_0002);

    // 10000060:    fec42703    lw x14,-20(x8)
    run_instruction!(rv);
    assert_eq!(rv.reg_file[14], 0x0000_002A /* 42 */);

    // 10000064:    200007b7    lui x15,0x20000
    run_instruction!(rv);
    assert_eq!(rv.reg_file[15], 0x2000_0000);

    // 10000068:    00478793    addi x15,x15,4 # 20000004 <_ebss+0x4>
    run_instruction!(rv);
    assert_eq!(rv.reg_file[15], 0x2000_0004);

    // 1000006c:    00e68733    add x14,x13,x14
    run_instruction!(rv);

    // 10000070:    00e7a023    sw x14,0(x15)
    run_instruction!(rv);
    assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0x0000_002C));

    // 10000074:    0000006f    jal x0,10000074 <main+0x30>
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0074);
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0074);
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0074);
}

#[test]
fn test_binary_2() {
    let instructions = load_binary("binary2.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    run_to_line!(rv, 0x1000_0034);
    assert_eq!(rv.reg_file[14], 5);
    assert_eq!(rv.reg_file[15], 8);

    run_to_line!(rv, 0x1000_0084);
    assert_eq!(rv.bus.read_word(0x2000_0000), Ok(0x0000_002A) /* 42 */);
    assert_eq!(rv.bus.read_word(0x2000_0004), Ok(0x0000_0001));
}

#[test]
fn test_binary_3() {
    let instructions = load_binary("binary3.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    run_to_line!(rv, 0x1000_0038);
    assert_eq!(rv.reg_file[15], 10);
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0028);
    assert_eq!(rv.reg_file[15], 10);

    run_to_line!(rv, 0x1000_0038);
    assert_eq!(rv.reg_file[15], 9);
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0028);

    run_to_line!(rv, 0x1000_0038);
    assert_eq!(rv.reg_file[15], 8);
    run_instruction!(rv);
    assert_eq!(rv.current_line(), 0x1000_0028);

    run_to_line!(rv, 0x1000_003C);
    assert_eq!(rv.reg_file[15], 0);
}

#[test]
fn test_binary_4() {
    let instructions = load_binary("binary4.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 5);
    assert_eq!(*rv.csr.instret.get(), 1);

    run_to_line!(rv, 0x1000_0018);
    assert_eq!(*rv.csr.cycles.get(), 35);
    assert_eq!(*rv.csr.instret.get(), 7);
    assert_eq!(rv.reg_file[15], 0);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 40);
    assert_eq!(*rv.csr.instret.get(), 8);
    assert_eq!(rv.reg_file[15], 7);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 45);
    assert_eq!(*rv.csr.instret.get(), 9);
    assert_eq!(rv.reg_file[15], 7);
}

#[test]
fn test_binary_5() {
    let instructions = load_binary("binary5.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 5);
    assert_eq!(*rv.csr.instret.get(), 1);

    run_to_line!(rv, 0x1000_0018);
    assert_eq!(*rv.csr.cycles.get(), 35);
    assert_eq!(*rv.csr.instret.get(), 7);
    assert_eq!(rv.reg_file[15], 0);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 40);
    assert_eq!(*rv.csr.instret.get(), 8);
    assert_eq!(rv.reg_file[15], 38);

    run_instruction!(rv);
    assert_eq!(*rv.csr.cycles.get(), 45);
    assert_eq!(*rv.csr.instret.get(), 9);
    assert_eq!(rv.reg_file[15], 38);
}

#[test]
#[should_panic /*(expected = "Unaligned read from address 0x203FFFEE") */]
fn test_binary_6() {
    let instructions = load_binary("binary6.bin");

    let mut rv = RV32ISystem::new();
    rv.bus.rom.load(instructions);

    // 10000080:    01010413    addi x8,x2,16
    run_to_line!(rv, 0x1000_0080);
    run_instruction!(rv);
}
