//! BIC encode/decode throughput on a few representative distributions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::bic;
use rand::{Rng, SeedableRng};

fn uniform_data(len: usize, max: u32) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    (0..len).map(|_| rng.gen_range(1..=max)).collect()
}

fn skewed_data(len: usize, common_max: u32, rare_max: u32, rare_freq: f64) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    (0..len)
        .map(|_| {
            if rng.gen::<f64>() < rare_freq {
                rng.gen_range(1..=rare_max)
            } else {
                rng.gen_range(1..=common_max)
            }
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bic");

    for &len in &[1_000usize, 10_000, 100_000] {
        let uniform_small = uniform_data(len, 100);
        let uniform_large = uniform_data(len, 10_000);
        let skewed = skewed_data(len, 10, 10_000, 0.05);
        let all_ones = vec![1u32; len];

        for (name, data) in [
            ("uniform_small", &uniform_small),
            ("uniform_large", &uniform_large),
            ("skewed_95_5", &skewed),
            ("all_ones", &all_ones),
        ] {
            let encoded = bic::encode(data);
            group.bench_with_input(
                BenchmarkId::new(format!("encode/{name}"), len),
                &(),
                |b, _| b.iter(|| black_box(bic::encode(black_box(data)))),
            );
            group.bench_with_input(
                BenchmarkId::new(format!("decode/{name}"), len),
                &(),
                |b, _| b.iter(|| black_box(bic::decode(black_box(&encoded)))),
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
