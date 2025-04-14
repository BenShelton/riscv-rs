#![allow(clippy::unusual_byte_groupings)]

use criterion::{Criterion, criterion_group, criterion_main};
use riscv::RV32ISystem;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("100 commands", |b| {
        b.iter(|| {
            let mut rv = RV32ISystem::new();
            rv.reg_file[1] = 0x0102_0304;
            rv.reg_file[2] = 0x0203_0405;
            rv.reg_file[10] = 0x8000_0000;
            rv.reg_file[11] = 0x0000_0001;

            rv.bus
                .rom
                .load(vec![0b000000000001_00001_000_00011_0010011; 100]);

            for _ in 0..500 {
                rv.cycle();
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
