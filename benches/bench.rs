use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    measurement::WallTime,
    BatchSize,
    BenchmarkGroup,
    Criterion,
    Throughput,
};
use super_simple_sat::Solver;
use std::fs;

criterion_group!(
    bench_solve,
    bench_uf20,
);
criterion_main!(bench_solve);

fn bench_uf20(c: &mut Criterion) {
    let input = fs::read("benches/inputs/uf20-01.cnf").unwrap();
    let solver = Solver::from_cnf(&mut &input[..]).unwrap();
    let mut g = c.benchmark_group("solve uf20");
    g.bench_function("01", |bencher| {
        bencher.iter_batched_ref(
            || solver.clone(),
            |solver| {
                assert!(solver.solve(vec![]));
            },
            BatchSize::SmallInput,
        )
    });
}
