use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    BatchSize,
    Criterion,
};
use std::{
    fs,
    path::Path,
};
use super_simple_sat::Solver;

criterion_group!(
    bench_solve,
    bench_3sat_v150_c645_sat,
    bench_3sat_v150_c645_unsat,
);
criterion_main!(bench_solve);

/// Returns the byte representation of all benchmarks found under the given path.
///
/// # Note
///
/// The benchmarks are returned alphabetically sorted by their file names.
fn collect_benchmarks_in_path<P>(path: P) -> Vec<Vec<u8>>
where
    P: AsRef<Path>,
{
    let mut dir_entries = fs::read_dir(path)
        .unwrap()
        .filter_map(|dir_entry| {
            match dir_entry {
                Ok(dir_entry) => {
                    let path = dir_entry.path();
                    if dir_entry.file_type().unwrap().is_file()
                        && path
                            .extension()
                            .map(|ext| ext == "cnf")
                            .unwrap_or_else(|| false)
                    {
                        let bytes = fs::read(dir_entry.path()).unwrap();
                        Some((path, bytes))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();
    dir_entries
        .sort_by(|(l_path, _), (r_path, _)| l_path.file_name().cmp(&r_path.file_name()));
    dir_entries
        .into_iter()
        .map(|(_path, bytes)| bytes)
        .collect::<Vec<_>>()
}

fn bench_3sat_v150_c645_sat(c: &mut Criterion) {
    let mut g = c.benchmark_group("3sat_v150_c645 (sat)");
    g.sample_size(10);
    for (n, input) in collect_benchmarks_in_path("cnf/uf150-645/sat/")
        .into_iter()
        .enumerate()
    {
        let solver = Solver::from_cnf(&mut &input[..]).unwrap();
        g.bench_function(n.to_string(), |bencher| {
            bencher.iter_batched_ref(
                || solver.clone(),
                |solver| {
                    let result = black_box(solver.solve(vec![]));
                    assert_eq!(result.map(|res| res.is_sat()), Ok(true));
                },
                BatchSize::SmallInput,
            )
        });
    }
}

fn bench_3sat_v150_c645_unsat(c: &mut Criterion) {
    let mut g = c.benchmark_group("3sat_v150_c645 (unsat)");
    g.sample_size(10);
    for (n, input) in collect_benchmarks_in_path("cnf/uf150-645/unsat/")
        .into_iter()
        .enumerate()
    {
        let solver = Solver::from_cnf(&mut &input[..]).unwrap();
        g.bench_function(n.to_string(), |bencher| {
            bencher.iter_batched_ref(
                || solver.clone(),
                |solver| {
                    let result = black_box(solver.solve(vec![]));
                    assert_eq!(result.map(|res| res.is_sat()), Ok(false));
                },
                BatchSize::SmallInput,
            )
        });
    }
}
