#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, Criterion};

pub fn bench1(c: &mut Criterion) {
}

criterion_group!(benches, bench1);
criterion_main!(benches);