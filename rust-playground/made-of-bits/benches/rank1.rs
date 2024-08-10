use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{BitVec, BitVecBuilder, MultiBitVec, MultiBitVecBuilder};
use rand::Rng;
// use rand::distributions::Uniform;
// use rand::prelude::Distribution;

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    // generate needles at uniform random
    // let unif = Uniform::new(0, haystack.last().unwrap() + 1);
    // or, generate needles in a narrow range
    // let unif = Uniform::new(100_000, 1_000_000);
    let mut g = c.benchmark_group("group a");

    let universe_size = 100_000;
    // density denominator; we will sweep x/denominator for x in 0..=denominator

    let denominator = 1000;
    for numerator in [1, 100, 700] {
        // 0.1%, 10%, 70% fill rate
        let mut b = made_of_bits::DenseBitVecBuilder::new(universe_size);
        for i in 0..universe_size {
            if rng.gen_ratio(numerator, denominator) {
                b.one(i)
            }
        }
        let v = b.build();

        // for now, generate a query vector that is 25% full
        let mut queries = vec![];
        for i in 0..universe_size {
            if rng.gen_ratio(25, 100) {
                queries.push(i)
            }
        }

        g.bench_function(BenchmarkId::new("rank1", numerator), |b| {
            b.iter(|| {
                let mut ret = 0;
                for q in queries.iter().copied() {
                    ret += v.rank1(q);
                }
                ret
            });
        });

        g.bench_function(BenchmarkId::new("rank1_batch", numerator), |b| {
            b.iter(|| v.rank1_batch(&queries).into_iter().sum::<u32>());
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
