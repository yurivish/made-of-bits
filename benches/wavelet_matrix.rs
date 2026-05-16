//! Wavelet matrix operations sweep: get, count, quantile, select, simple_majority,
//! count_batch, locate_batch, quantile_batch over `(length, alphabet)` configurations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{DenseBitVec, WaveletMatrix};
use rand::{Rng, SeedableRng};

fn random_data(len: usize, max_symbol: u32, seed: u64) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    (0..len).map(|_| rng.gen_range(0..=max_symbol)).collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("wavelet_matrix");
    group.sample_size(20);

    for &(len, max_sym) in &[(10_000usize, 255u32), (100_000, 255), (100_000, 4095)] {
        let data = random_data(len, max_sym, 42);
        let wm = WaveletMatrix::<DenseBitVec>::new(
            data.clone(),
            max_sym,
            Default::default(),
            None,
        );
        let len_u32 = data.len() as u32;
        let label = format!("{len}_{max_sym}");

        // Reusable query sets.
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let indices: Vec<u32> = (0..1000).map(|_| rng.gen_range(0..len_u32)).collect();
        let symbols: Vec<u32> = (0..100).map(|_| rng.gen_range(0..=max_sym)).collect();
        let half_range = 0..(len_u32 / 2);

        // get
        group.bench_function(BenchmarkId::new("get", &label), |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for &i in &indices {
                    acc = acc.wrapping_add(wm.get(i) as u64);
                }
                black_box(acc)
            });
        });

        // count over the full range
        group.bench_function(BenchmarkId::new("count", &label), |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for &s in &symbols {
                    acc = acc.wrapping_add(wm.count(0..len_u32, s) as u64);
                }
                black_box(acc)
            });
        });

        // quantile mid-range
        group.bench_function(BenchmarkId::new("quantile", &label), |b| {
            b.iter(|| {
                let (s, _) = wm.quantile(half_range.clone(), (len_u32 / 4) as u32);
                black_box(s)
            });
        });

        // simple_majority — Quantile + threshold check
        group.bench_function(BenchmarkId::new("simple_majority", &label), |b| {
            b.iter(|| black_box(wm.simple_majority(0..len_u32)));
        });

        // quantile_batch — single range, many ks
        let mut ks: Vec<u32> = (0..50).map(|i| i * (len_u32 / 50)).collect();
        ks.sort_unstable();
        group.bench_function(BenchmarkId::new("quantile_batch", &label), |b| {
            let ks = ks.clone();
            b.iter_batched(
                || ks.clone(),
                |mut ks| {
                    let v = wm.quantile_batch(0..len_u32, &mut ks);
                    black_box(v)
                },
                criterion::BatchSize::SmallInput,
            );
        });

        // count_batch — single range, many symbol ranges
        let symbol_ranges: Vec<std::ops::RangeInclusive<u32>> =
            (0..10).map(|i| (i * 10)..=((i + 1) * 10)).collect();
        group.bench_function(BenchmarkId::new("count_batch", &label), |b| {
            b.iter(|| black_box(wm.count_batch(0..len_u32, &symbol_ranges)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
