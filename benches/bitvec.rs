//! Rank/select/batch/ones sweep across Dense, Sparse, RLE.
//!
//! Two select-batch workloads cover both regimes:
//! - `select1` / `select1_batch`        — dense, sorted, consecutive query indices.
//!   The select-sample table is hot; the hint saves only the lookup itself.
//! - `select1_sparse` / `select1_batch_sparse` — sorted indices spread across
//!   the whole range. The sample table sees a cold lookup per query in the
//!   baseline, which the hint elides for nearby queries.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{
    BitVec, BitVecBuilder, BitVecBuilderOf, DenseBitVecBuilder, RLEBitVecBuilder,
    SparseBitVecBuilder,
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

/// Sorted select1 queries spread across `[0, num_ones)`. Each query lands in a
/// different select-sample bucket, exercising the cold-cache path.
fn sparse_select_queries(num_ones: u32, count: usize, seed: u64) -> Vec<u32> {
    if num_ones == 0 {
        return Vec::new();
    }
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut q: Vec<u32> = (0..count).map(|_| rng.gen_range(0..num_ones)).collect();
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
            let num_ones = ones.len() as u32;

            // Dense select queries: consecutive 1-bit indices in [0, num_ones/10).
            let select_queries_dense: Vec<u32> = (0..num_ones / 10).collect();
            // Sparse select queries: 1000 random sorted 1-bit indices in [0, num_ones).
            let select_queries_sparse = sparse_select_queries(num_ones, 1000, 11);

            let dense = DenseBitVecBuilder::from_ones(universe, Default::default(), &ones);
            let rle = RLEBitVecBuilder::from_ones(universe, Default::default(), &ones);
            // SparseBitVec is a MultiBitVec; wrap via BitVecOf for the BitVec trait.
            let sparse = BitVecBuilderOf::<SparseBitVecBuilder>::from_ones(
                universe,
                Default::default(),
                &ones,
            );

            let suffix = format!("{universe}_d{density_pct}");

            // rank1 (per-query)
            group.bench_function(BenchmarkId::new("dense/rank1", &suffix), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &queries {
                        acc = acc.wrapping_add(dense.rank1(q) as u64);
                    }
                    black_box(acc)
                });
            });
            group.bench_function(BenchmarkId::new("sparse/rank1", &suffix), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &queries {
                        acc = acc.wrapping_add(sparse.rank1(q) as u64);
                    }
                    black_box(acc)
                });
            });
            group.bench_function(BenchmarkId::new("rle/rank1", &suffix), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &queries {
                        acc = acc.wrapping_add(rle.rank1(q) as u64);
                    }
                    black_box(acc)
                });
            });

            // rank1_batch (hint propagation across monotone queries)
            group.bench_function(BenchmarkId::new("dense/rank1_batch", &suffix), |b| {
                b.iter_batched(
                    || queries.clone(),
                    |mut qs| {
                        dense.rank1_batch(&mut qs);
                        black_box(qs)
                    },
                    criterion::BatchSize::SmallInput,
                );
            });
            group.bench_function(BenchmarkId::new("sparse/rank1_batch", &suffix), |b| {
                b.iter_batched(
                    || queries.clone(),
                    |mut qs| {
                        sparse.rank1_batch(&mut qs);
                        black_box(qs)
                    },
                    criterion::BatchSize::SmallInput,
                );
            });

            if num_ones == 0 {
                continue;
            }

            // Dense select workload (cache-warm).
            group.bench_function(BenchmarkId::new("dense/select1", &suffix), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &select_queries_dense {
                        acc = acc.wrapping_add(dense.select1(q).unwrap_or(0) as u64);
                    }
                    black_box(acc)
                });
            });
            group.bench_function(
                BenchmarkId::new("dense/select1_batch", &suffix),
                |b| {
                    b.iter_batched(
                        || select_queries_dense.clone(),
                        |mut qs| {
                            dense.select1_batch(&mut qs);
                            black_box(qs)
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );

            // Sparse select workload (cache-cold). Same query count for both so
            // the comparison is apples-to-apples.
            group.bench_function(BenchmarkId::new("dense/select1_sparse", &suffix), |b| {
                b.iter(|| {
                    let mut acc = 0u64;
                    for &q in &select_queries_sparse {
                        acc = acc.wrapping_add(dense.select1(q).unwrap_or(0) as u64);
                    }
                    black_box(acc)
                });
            });
            group.bench_function(
                BenchmarkId::new("dense/select1_batch_sparse", &suffix),
                |b| {
                    b.iter_batched(
                        || select_queries_sparse.clone(),
                        |mut qs| {
                            dense.select1_batch(&mut qs);
                            black_box(qs)
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );

            // ones() iterator — block-walking, no per-query overhead.
            group.bench_function(BenchmarkId::new("dense/ones", &suffix), |b| {
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
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
