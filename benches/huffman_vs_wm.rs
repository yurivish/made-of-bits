//! Compare `HuffmanWaveletMatrix` against a plain `WaveletMatrix` on identical Zipf-
//! distributed data. The point of HuffmanWM is to compress towards first-order entropy
//! while preserving correctness on overlapping queries — these benchmarks measure how
//! the two structures perform on `get`, `count`, and `select` under skewed distributions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::huffmanwaveletmatrix::HuffmanWaveletMatrix;
use made_of_bits::{DenseBitVec, WaveletMatrix};
use rand::{Rng, SeedableRng};

/// Generate a Zipf-distributed symbol stream: symbol `s` appears with frequency ~ 1/(s+1).
/// Skewed enough that the rare symbols' code lengths blow up — HuffmanWM should compress
/// the structure aggressively while plain WM keeps every level at full length.
fn zipf_data(len: usize, max_symbol: u32) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let cum: Vec<f64> = {
        let weights: Vec<f64> = (1..=max_symbol).map(|s| 1.0 / s as f64).collect();
        let total: f64 = weights.iter().sum();
        let mut acc = 0.0;
        weights
            .iter()
            .map(|w| {
                acc += w / total;
                acc
            })
            .collect()
    };
    (0..len)
        .map(|_| {
            let r: f64 = rng.gen();
            cum.iter()
                .position(|&c| r < c)
                .map(|i| i as u32 + 1)
                .unwrap_or(max_symbol)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("wm_vs_hwm");

    for &len in &[10_000usize, 100_000] {
        for &max_symbol in &[256u32, 4096] {
            let data = zipf_data(len, max_symbol);
            let wm =
                WaveletMatrix::<DenseBitVec>::new(data.clone(), max_symbol, Default::default(), None);
            let hwm = HuffmanWaveletMatrix::new(&data, Default::default());

            // `get` over a fixed sequence of indices.
            let indices: Vec<u32> = (0..1000)
                .map(|i| ((i as u64 * 9301 + 49297) % data.len() as u64) as u32)
                .collect();
            group.bench_with_input(
                BenchmarkId::new("wm/get", format!("{len}_{max_symbol}")),
                &(),
                |b, _| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for &i in &indices {
                            acc = acc.wrapping_add(wm.get(i) as u64);
                        }
                        black_box(acc)
                    })
                },
            );
            group.bench_with_input(
                BenchmarkId::new("hwm/get", format!("{len}_{max_symbol}")),
                &(),
                |b, _| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for &i in &indices {
                            acc = acc.wrapping_add(hwm.get(i) as u64);
                        }
                        black_box(acc)
                    })
                },
            );

            // `count` for a few popular symbols.
            let symbols: Vec<u32> = (1..=8).collect();
            group.bench_with_input(
                BenchmarkId::new("wm/count", format!("{len}_{max_symbol}")),
                &(),
                |b, _| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for &s in &symbols {
                            acc = acc.wrapping_add(wm.count(0..data.len() as u32, s) as u64);
                        }
                        black_box(acc)
                    })
                },
            );
            group.bench_with_input(
                BenchmarkId::new("hwm/count", format!("{len}_{max_symbol}")),
                &(),
                |b, _| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for &s in &symbols {
                            acc = acc.wrapping_add(hwm.count(0..data.len() as u32, s) as u64);
                        }
                        black_box(acc)
                    })
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
