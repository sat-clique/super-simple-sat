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
fn simple_works() {
    let mut solver = Solver::from_cnf(&mut &b"1 2"[..]).unwrap();
    assert_eq!(solver.solve(vec![]), true);
}

#[test]
fn solve_empty_problem_works() {
    let mut solver = Solver::default();
    assert!(solver.solve(vec![]));
}

fn clause(lits: &[Literal]) -> Clause {
    Clause::new(lits.into_iter().copied()).unwrap()
}

#[test]
fn solve_problem_with_single_unit_clause() {
    let mut solver = Solver::default();
    let a = solver.new_literal();
    solver.consume_clause(clause(&[a]));
    assert!(solver.solve(vec![]));
}

#[test]
#[rustfmt::skip]
fn solve_problem_with_non_contradictory_unit_clauses() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[2]]));
    solver.consume_clause(clause(&[ vars[4]]));
    solver.consume_clause(clause(&[!vars[5]]));
    assert!(solver.solve(vec![]));
}

#[test]
#[rustfmt::skip]
fn solve_problem_with_contradictory_unit_clauses() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[2]]));
    solver.consume_clause(clause(&[ vars[4]]));
    solver.consume_clause(clause(&[!vars[4]]));
    let result = solver.solve(vec![]);
    assert!(!result);
}

#[test]
#[rustfmt::skip]
fn test_solve_satisfiable_3sat_problem() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]]));
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]]));
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]]));
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]]));
    let result = solver.solve(vec![]);
    assert!(result);
}

#[test]
#[rustfmt::skip]
fn test_unsatisfiable_2sat_problem() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[!vars[1],  vars[3]]));
    solver.consume_clause(clause(&[!vars[3],  vars[8]]));
    solver.consume_clause(clause(&[!vars[8], !vars[1]]));
    solver.consume_clause(clause(&[ vars[4],  vars[1]]));
    solver.consume_clause(clause(&[!vars[4],  vars[7]]));
    solver.consume_clause(clause(&[!vars[7], !vars[4]]));
    let result = solver.solve(vec![]);
    assert!(!result);
}

#[test]
#[rustfmt::skip]
fn test_solve_3sat_problem_with_satisfiable_assumptions() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]]));
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]]));
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]]));
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]]));
    let result = solver.solve(vec![vars[1], vars[7], vars[6]]);
    assert!(result);
}

#[test]
#[rustfmt::skip]
fn test_solve_3sat_problem_with_unsatisfiable_assumptions() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]]));
    solver.consume_clause(clause(&[ vars[1], !vars[7], !vars[5]]));
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]]));
    solver.consume_clause(clause(&[!vars[9], !vars[6],  vars[1]]));
    let result = solver.solve(vec![!vars[1], !vars[3], vars[7]]);
    assert!(!result);
}

#[test]
#[rustfmt::skip]
fn test_get_forced_assignment() {
    let mut solver = Solver::default();
    let vars = (0..10).map(|_| solver.new_literal()).collect::<Vec<_>>();
    solver.consume_clause(clause(&[ vars[1],  vars[3],  vars[5]]));
    solver.consume_clause(clause(&[!vars[1], !vars[7],  vars[5]]));
    solver.consume_clause(clause(&[!vars[3], !vars[7], !vars[0]]));
    solver.consume_clause(clause(&[!vars[9], !vars[6], !vars[1]]));
    let result = solver.solve(vec![vars[1], vars[7], vars[6]]);
    assert!(result);
    let model = solver.last_model().unwrap();
    assert!( model.is_satisfied( vars[1]).unwrap());
    assert!(!model.is_satisfied(!vars[1]).unwrap());
    assert!( model.is_satisfied( vars[7]).unwrap());
    assert!(!model.is_satisfied(!vars[7]).unwrap());
    assert!( model.is_satisfied( vars[6]).unwrap());
    assert!(!model.is_satisfied(!vars[6]).unwrap());
    assert!( model.is_satisfied( vars[5]).unwrap());
    assert!(!model.is_satisfied(!vars[5]).unwrap());
    assert!(!model.is_satisfied( vars[9]).unwrap());
    assert!( model.is_satisfied(!vars[9]).unwrap());
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
    assert!(result);
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
fn test_uf100_430_sat() {
    for (n, input) in collect_tests_in_path("cnf/uf100-430/sat/")
        .into_iter()
        .enumerate()
    {
        let mut solver = Solver::from_cnf(&mut &input[..]).unwrap();
        assert!(solver.solve(vec![]), "failed at unsat uf100-430/{}", n);
    }
}

#[test]
fn test_uf100_430_unsat() {
    for (n, input) in collect_tests_in_path("cnf/uf100-430/unsat/")
        .into_iter()
        .enumerate()
    {
        let mut solver = Solver::from_cnf(&mut &input[..]).unwrap();
        assert!(!solver.solve(vec![]), "failed at unsat uf100-430/{}", n);
    }
}
