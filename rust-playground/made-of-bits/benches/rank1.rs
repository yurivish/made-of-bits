use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{BitVec, BitVecBuilder, MultiBitVec, MultiBitVecBuilder};
use rand::Rng;
// use rand::distributions::Uniform;
// use rand::prelude::Distribution;

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut g = c.benchmark_group("group a");

    let universe_size = 1_000_000;

    let denominator = 1000; // density denominator
    for numerator in [1, 100, 700] {
        // 0.1%, 10%, 70% fill rate
        let mut b = made_of_bits::DenseBitVecBuilder::new(universe_size);
        for i in 0..universe_size {
            if rng.gen_ratio(numerator, denominator) {
                b.one(i)
            }
        }
        let v = b.build();

        // for now, generate a query vector that is 5% full
        let mut queries = vec![];
        for i in 0..universe_size {
            if rng.gen_ratio(5, 100) {
                queries.push(i)
            }
        }

        g.bench_function(BenchmarkId::new("rank1", numerator), |b| {
            b.iter(|| {
                let mut ret: u32 = 0;
                for q in queries.iter().copied() {
                    ret = ret.wrapping_add(v.rank1(q));
                }
                ret
            });
        });

        g.bench_function(BenchmarkId::new("rank1_batch", numerator), |b| {
            b.iter(|| {
                let mut ret: u32 = 0;
                for result in v.rank1_batch(&queries).into_iter() {
                    ret = ret.wrapping_add(result);
                }
                ret
            });
        });
    }
    g.finish();
}

criterion_group! {
    name    = benches;
    config  = Criterion::default().warm_up_time(Duration::from_secs(1));
    targets = criterion_benchmark
}
criterion_main!(benches);
