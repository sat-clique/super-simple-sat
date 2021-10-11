use core::{
    fmt,
    fmt::{
        Display,
        Formatter,
    },
};
use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    BatchSize,
    BenchmarkId,
    Criterion,
};
use s3sat_solver::Solver;
use std::{
    fs,
    path::Path,
};

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

/// The kind of the SAT problem.
#[derive(Debug, Copy, Clone)]
pub enum ProblemKind {
    /// Randomly generated 3-SAT problem instance.
    Random3Sat,
}

impl Display for ProblemKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Random3Sat => write!(f, "Random 3-SAT"),
        }
    }
}

/// The benchmark parameters.
pub struct BenchParams {
    problem_kind: ProblemKind,
    satisfiable: Satisfiability,
    len_clauses: usize,
    len_literals: usize,
    instance: usize,
}

/// The known satisfiability of a SAT benchmark instance.
#[derive(Debug)]
pub enum Satisfiability {
    /// The benchmark instance is satisfiable.
    Sat,
    /// The benchmark instance is unsatisfiable.
    Unsat,
}

impl Display for Satisfiability {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Sat => write!(f, "satisfiable"),
            Self::Unsat => write!(f, "unsatisfiable"),
        }
    }
}

impl BenchParams {
    /// Creates new benchmark parameters.
    pub fn new(
        problem_kind: ProblemKind,
        satisfiable: Satisfiability,
        len_clauses: usize,
        len_literals: usize,
        instance: usize,
    ) -> Self {
        Self {
            problem_kind,
            satisfiable,
            len_clauses,
            len_literals,
            instance,
        }
    }
}

impl Display for BenchParams {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{} (#clauses = {}, #literals = {}, {}) #{}",
            self.problem_kind,
            self.len_clauses,
            self.len_literals,
            self.satisfiable,
            self.instance
        )
    }
}

fn bench_3sat_v150_c645_sat(c: &mut Criterion) {
    let mut g = c.benchmark_group("Solver::solve");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    for (n, input) in collect_benchmarks_in_path("../../cnf/uf150-645/sat/")
        .into_iter()
        .enumerate()
    {
        let solver = Solver::from_cnf(&mut &input[..]).unwrap();
        let param =
            BenchParams::new(ProblemKind::Random3Sat, Satisfiability::Sat, 650, 150, n);
        g.bench_function(BenchmarkId::from_parameter(param), |bencher| {
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
    let mut g = c.benchmark_group("Solver::solve");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    for (n, input) in collect_benchmarks_in_path("../../cnf/uf150-645/unsat/")
        .into_iter()
        .enumerate()
    {
        let solver = Solver::from_cnf(&mut &input[..]).unwrap();
        let param =
            BenchParams::new(ProblemKind::Random3Sat, Satisfiability::Unsat, 650, 150, n);
        g.bench_function(BenchmarkId::from_parameter(param), |bencher| {
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
