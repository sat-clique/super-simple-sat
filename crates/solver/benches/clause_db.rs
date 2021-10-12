use core::iter::repeat;
use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    BatchSize,
    BenchmarkId,
    Criterion,
};
use s3sat_solver::{
    clause_db::{
        ClauseDatabase,
        ClauseRef,
    },
    Literal,
};
use std::fmt::{
    self,
    Display,
};

criterion_group!(
    bench_clause_database,
    alloc,
    resolve,
    resolve_mut,
    iter,
    remove_clause,
    gc
);
criterion_main!(bench_clause_database);

/// Returns a clause database with an amount of clauses of a given size.
fn bench_database(params: BenchParams) -> (Vec<ClauseRef>, ClauseDatabase) {
    let mut db = ClauseDatabase::default();
    let literals = (1i32..params.len_literals as i32)
        .into_iter()
        .map(Literal::from);
    let clause_refs = repeat(literals)
        .take(params.len_clauses)
        .map(|lits| db.alloc(lits))
        .collect::<Vec<_>>();
    (clause_refs, db)
}

/// A benchmark identifier.
#[derive(Debug, Copy, Clone)]
pub struct BenchParams {
    /// The amount of clauses for the benchmark.
    len_clauses: usize,
    /// The amount of literals per clause for the benchmark.
    len_literals: usize,
}

impl BenchParams {
    /// Creates a new benchmark identifier.
    pub fn new(len_clauses: usize, len_literals: usize) -> Self {
        Self {
            len_clauses,
            len_literals,
        }
    }
}

impl Display for BenchParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(#clauses = {}, #literals = {})",
            self.len_clauses, self.len_literals
        )
    }
}

fn alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::alloc");
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        bencher.iter_batched_ref(
            ClauseDatabase::default,
            |db| {
                let literals = [1, 2, 3, 4, 5].map(Literal::from);
                for lits in repeat(literals).take(count_clauses) {
                    black_box(db.alloc(lits));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

fn resolve(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::resolve");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        let (clause_refs, db) = bench_database(params);
        bencher.iter(|| {
            for cref in &clause_refs {
                let _ = black_box(db.resolve(*cref).unwrap());
            }
        });
    });
}

fn resolve_mut(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::resolve_mut");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        let (clause_refs, mut db) = bench_database(params);
        bencher.iter(|| {
            for cref in &clause_refs {
                let _ = black_box(db.resolve_mut(*cref).unwrap());
            }
        });
    });
}

fn iter(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::iter");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        let (_, db) = bench_database(params);
        bencher.iter(|| {
            for clause in &db {
                let _ = black_box(clause);
            }
        });
    });
}

fn remove_clause(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::remove_clause");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        let (clause_refs, db) = bench_database(params);
        bencher.iter_batched_ref(
            || db.clone(),
            |db| {
                for cref in &clause_refs {
                    let _ = black_box(db.remove_clause(*cref));
                }
            },
            BatchSize::SmallInput,
        );
    });
}

fn gc(c: &mut Criterion) {
    let mut g = c.benchmark_group("ClauseDatabase::gc");
    g.sample_size(10);
    g.sampling_mode(criterion::SamplingMode::Flat);
    let count_clauses = 10_000;
    let clause_size = 5;
    let params = BenchParams::new(count_clauses, clause_size);
    g.bench_function(BenchmarkId::from_parameter(params), |bencher| {
        let (clause_refs, db) = bench_database(params);
        bencher.iter_batched_ref(
            || {
                let mut db = db.clone();
                for cref in clause_refs.iter().step_by(1) {
                    db.remove_clause(*cref);
                }
                db
            },
            |db| {
                let _ = black_box(db.gc(|from, into| {
                    black_box((from, into));
                }));
            },
            BatchSize::SmallInput,
        )
    });
}
