use solver::{
    SolveResult,
    Solver,
};
use std::{
    fs,
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(name = "input .cnf file", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt = Opt::from_args();
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
