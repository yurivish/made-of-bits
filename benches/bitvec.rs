//! Rank/select/batch/ones sweep across Dense, Sparse, RLE at three universe sizes and
//! three densities. Mirrors `madeofbits/cmd/bench/main.go` in spirit, scaled down for
//! Criterion.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{
    BitVec, BitVecBuilder, DenseBitVec, DenseBitVecBuilder, RLEBitVec, RLEBitVecBuilder,
};
use rand::{Rng, SeedableRng};

fn random_ones(universe: u32, density: f64, seed: u64) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut ones: Vec<u32> = (0..universe)
        .filter(|_| rng.gen::<f64>() < density)
        .collect();
    ones.sort_unstable();
    ones.dedup();
    ones
}

fn random_queries(universe: u32, count: usize, seed: u64) -> Vec<u32> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut q: Vec<u32> = (0..count).map(|_| rng.gen_range(0..universe)).collect();
    q.sort_unstable();
    q
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitvec");
    group.sample_size(20);

    for &universe in &[10_000u32, 100_000, 1_000_000] {
        for &density_pct in &[1u32, 10, 50] {
            let density = density_pct as f64 / 100.0;
            let ones = random_ones(universe, density, 42);
            let queries = random_queries(universe, 1000, 7);
            let select_queries: Vec<u32> = (0..ones.len() as u32 / 10).collect();

            let dense = DenseBitVecBuilder::from_ones(universe, Default::default(), &ones);
            let rle = RLEBitVecBuilder::from_ones(universe, Default::default(), &ones);

            // Sparse not built via BitVecBuilder (it's a MultiBitVec); skip for this sweep.
            // The existing rank1.rs covers SparseBitVec specifically.

            let label = |kind: &str, op: &str| format!("{kind}/{op}/{universe}_d{density_pct}");

            // rank1
            group.bench_function(BenchmarkId::new("dense/rank1", &label("", "")), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &queries {
                        acc = acc.wrapping_add(dense.rank1(q) as u64);
                    }
                    black_box(acc)
                });
            });
            group.bench_function(BenchmarkId::new("rle/rank1", &label("", "")), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &queries {
                        acc = acc.wrapping_add(rle.rank1(q) as u64);
                    }
                    black_box(acc)
                });
            });

            // rank1_batch
            group.bench_function(BenchmarkId::new("dense/rank1_batch", &label("", "")), |b| {
                let qs = queries.clone();
                b.iter_batched(
                    || qs.clone(),
                    |mut qs| {
                        dense.rank1_batch(&mut qs);
                        black_box(qs)
                    },
                    criterion::BatchSize::SmallInput,
                );
            });

            // select1
            if !ones.is_empty() {
                group.bench_function(BenchmarkId::new("dense/select1", &label("", "")), |b| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for &q in &select_queries {
                            acc = acc.wrapping_add(dense.select1(q).unwrap_or(0) as u64);
                        }
                        black_box(acc)
                    });
                });

                // select1_batch — new in P2.2
                group.bench_function(
                    BenchmarkId::new("dense/select1_batch", &label("", "")),
                    |b| {
                        let qs = select_queries.clone();
                        b.iter_batched(
                            || qs.clone(),
                            |mut qs| {
                                dense.select1_batch(&mut qs);
                                black_box(qs)
                            },
                            criterion::BatchSize::SmallInput,
                        );
                    },
                );

                // ones() iterator — new in P2.2
                group.bench_function(BenchmarkId::new("dense/ones", &label("", "")), |b| {
                    b.iter(|| {
                        let mut acc = 0u64;
                        for pos in dense.ones() {
                            acc = acc.wrapping_add(pos as u64);
                        }
                        black_box(acc)
                    });
                });
            }
        }
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
