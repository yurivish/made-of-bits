use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use made_of_bits::{MultiBitVec, MultiBitVecBuilder};
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
    let denominator = 5;
    // test nonempty, nonfull bitvecs
    for numerator in 1..denominator {
        let mut b = made_of_bits::ArrayBitVecBuilder::new(universe_size);
        for i in 0..universe_size {
            if rng.gen_ratio(numerator, denominator) {
                b.ones(i, 1)
            }
        }
        let v = b.build();

        let inds: Vec<u32> = (0..universe_size).step_by(11).collect();

        g.bench_function(BenchmarkId::new("rank1", numerator), |b| {
            b.iter(|| {
                let mut ret = 0;
                for ind in inds.iter().copied() {
                    ret += v.rank1(ind);
                }
                ret
            })
        });
    }
    g.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
