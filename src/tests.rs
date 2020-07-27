use crate::{
    Clause,
    Literal,
    Solver,
    VarAssignment,
    Variable,
};
use std::{
    fs,
    path::Path,
};

#[test]
fn simple_sat_works() {
    let mut solver = Solver::from_cnf(
        &mut &br"
        p cnf 2 1
        1 2 0
    "[..],
    )
    .unwrap();
    assert_eq!(solver.solve(vec![]), Ok(true));
}

#[test]
fn simple_unsat_works() {
    let mut solver = Solver::from_cnf(
        &mut &br"
        p cnf 2 4
        1 2 0
        -1 -2 0
        1 -2 0
        -1 2 0
    "[..],
    )
    .unwrap();
    assert_eq!(solver.solve(vec![]), Ok(false));
}

#[test]
fn solve_empty_problem_works() {
    let mut solver = Solver::default();
    assert_eq!(solver.solve(vec![]), Ok(true));
}

fn clause(lits: &[Literal]) -> Clause {
    Clause::new(lits.into_iter().copied()).unwrap()
}

#[test]
fn solve_problem_with_single_unit_clause() {
    let mut solver = Solver::default();
    let a = solver.new_literal().unwrap();
    solver.consume_clause(clause(&[a])).unwrap();
    assert_eq!(solver.solve(vec![]), Ok(true));
}

#[test]
#[rustfmt::skip]
fn solve_problem_with_non_contradictory_unit_clauses() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[2]])).unwrap();
    solver.consume_clause(clause(&[ vars[4]])).unwrap();
    solver.consume_clause(clause(&[!vars[5]])).unwrap();
    assert_eq!(solver.solve(vec![]), Ok(true));
}

#[test]
#[rustfmt::skip]
fn solve_problem_with_contradictory_unit_clauses() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[2]])).unwrap();
    solver.consume_clause(clause(&[ vars[4]])).unwrap();
    solver.consume_clause(clause(&[!vars[4]])).unwrap();
    let result = solver.solve(vec![]);
    assert_eq!(result, Ok(false));
}

#[test]
#[rustfmt::skip]
fn test_solve_satisfiable_3sat_problem() {
    let mut solver = Solver::default();    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]])).unwrap();
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]])).unwrap();
    let result = solver.solve(vec![]);
    assert_eq!(result, Ok(true));
}

#[test]
#[rustfmt::skip]
fn test_unsatisfiable_2sat_problem() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[!vars[1],  vars[3]])).unwrap();
    solver.consume_clause(clause(&[!vars[3],  vars[8]])).unwrap();
    solver.consume_clause(clause(&[!vars[8], !vars[1]])).unwrap();
    solver.consume_clause(clause(&[ vars[4],  vars[1]])).unwrap();
    solver.consume_clause(clause(&[!vars[4],  vars[7]])).unwrap();
    solver.consume_clause(clause(&[!vars[7], !vars[4]])).unwrap();
    let result = solver.solve(vec![]);
    assert_eq!(result, Ok(false));
}

#[test]
#[rustfmt::skip]
fn test_solve_3sat_problem_with_satisfiable_assumptions() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]])).unwrap();
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]])).unwrap();
    let result = solver.solve(vec![vars[1], vars[7], vars[6]]);
    assert_eq!(result, Ok(true));
}

#[test]
#[rustfmt::skip]
fn test_solve_3sat_problem_with_unsatisfiable_assumptions() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[ vars[1], !vars[7], !vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]])).unwrap();
    solver.consume_clause(clause(&[!vars[9], !vars[6],  vars[1]])).unwrap();
    let result = solver.solve(vec![!vars[1], !vars[3], vars[7]]);
    assert_eq!(result, Ok(false));
}

#[test]
#[rustfmt::skip]
fn test_get_forced_assignment() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Result<Vec<_>, _>>().unwrap();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]])).unwrap();
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]])).unwrap();
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]])).unwrap();
    let result = solver.solve(vec![vars[1], vars[7], vars[6]]);
    assert_eq!(result, Ok(true));
    let model = solver.last_model().unwrap();
    assert_eq!(model.is_satisfied( vars[1]), Ok(Some(true)));
    assert_eq!(model.is_satisfied(!vars[1]), Ok(Some(false)));
    assert_eq!(model.is_satisfied( vars[7]), Ok(Some(true)));
    assert_eq!(model.is_satisfied(!vars[7]), Ok(Some(false)));
    assert_eq!(model.is_satisfied( vars[6]), Ok(Some(true)));
    assert_eq!(model.is_satisfied(!vars[6]), Ok(Some(false)));
    assert_eq!(model.is_satisfied( vars[5]), Ok(Some(true)));
    assert_eq!(model.is_satisfied(!vars[5]), Ok(Some(false)));
    assert_eq!(model.is_satisfied( vars[9]), Ok(Some(false)));
    assert_eq!(model.is_satisfied(!vars[9]), Ok(Some(true)));
}

#[test]
fn test_cnf_input() {
    let cnf_input = br"
        p cnf 10 4
        1 3 5 0
        -2 -8 6 0
        -4 -8 -1 0
        -10 -7 -2 0
    ";
    let mut solver = Solver::from_cnf(&mut &cnf_input[..]).unwrap();
    let assumption_1 = Variable::from_index(0)
        .unwrap()
        .into_literal(VarAssignment::True);
    let assumption_2 = Variable::from_index(6)
        .unwrap()
        .into_literal(VarAssignment::True);
    let assumption_3 = Variable::from_index(5)
        .unwrap()
        .into_literal(VarAssignment::True);
    let result = solver.solve(vec![assumption_1, assumption_2, assumption_3]);
    assert_eq!(result, Ok(true));
}

/// Returns the byte representation of all benchmarks found under the given path.
///
/// # Note
///
/// The benchmarks are returned alphabetically sorted by their file names.
fn collect_tests_in_path<P>(path: P) -> Vec<Vec<u8>>
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

#[test]
fn test_3sat_v100_c430_sat() {
    for (n, input) in collect_tests_in_path("cnf/uf100-430/sat/")
        .into_iter()
        .enumerate()
    {
        let mut solver = Solver::from_cnf(&mut &input[..]).unwrap();
        assert_eq!(
            solver.solve(vec![]),
            Ok(true),
            "failed at unsat uf100-430/{}",
            n
        );
    }
}

#[test]
fn test_3sat_v100_c430_unsat() {
    for (n, input) in collect_tests_in_path("cnf/uf100-430/unsat/")
        .into_iter()
        .enumerate()
    {
        let mut solver = Solver::from_cnf(&mut &input[..]).unwrap();
        assert_eq!(
            solver.solve(vec![]),
            Ok(false),
            "failed at unsat uf100-430/{}",
            n
        );
    }
}
