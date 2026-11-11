#![forbid(unsafe_code)]

use clap::Parser;
use solver::{
    SolveResult,
    Solver,
};
use std::{
    fs,
    path::PathBuf,
};

#[derive(Parser, Debug)]
struct Opt {
    #[arg(value_name = "input .cnf file")]
    input: PathBuf,
}

fn main() {
    let opt = Opt::parse();
    let cnf_contents =
        fs::read(opt.input).expect("couldn't read provided input .cnf file");
    let mut solver = Solver::from_cnf(&mut &cnf_contents[..])
        .expect("couldn't properly decode provided input .cnf file");
    println!("start solving ...");
    let result = solver
        .solve(vec![])
        .expect("encountered errors during solving");
    match result {
        SolveResult::Sat(model) => {
            println!("SAT\nmodel = {}", model);
        }
        SolveResult::Unsat => {
            println!("UNSAT");
        }
    }
}
