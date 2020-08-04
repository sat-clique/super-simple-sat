#![forbid(unsafe_code)]
#![allow(clippy::len_without_is_empty)]

mod assignment2;
mod builder;
mod clause_db;
mod decider;
mod literal;
mod literal_chunk;
mod utils;

#[cfg(test)]
mod tests;

use crate::{
    assignment2::{
        Assignment as Assignment2,
        AssignmentError,
        LastModel as LastModel2,
        Model as Model2,
        PropagationResult as PropagationResult2,
    },
    builder::SolverBuilder,
    clause_db::ClauseDb,
    decider::Decider,
};
pub use crate::{
    clause_db::Clause,
    literal::{
        Literal,
        VarAssignment,
        Variable,
    },
    literal_chunk::{
        LiteralChunk,
        LiteralChunkIter,
    },
    utils::Bool,
};
use cnf_parser::{
    Error as CnfError,
    Input,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Other(&'static str),
    Assignment(AssignmentError),
    Bounded(utils::Error),
    Conflict,
    InvalidLiteralChunkRange,
    InvalidLiteralChunkStart,
    InvalidLiteralChunkEnd,
    TooManyVariablesInUse,
    InvalidDecisionId,
    InvalidDecisionStart,
    InvalidDecisionEnd,
    InvalidSizeIncrement,
}

impl From<utils::Error> for Error {
    fn from(err: utils::Error) -> Self {
        Self::Bounded(err)
    }
}

impl From<AssignmentError> for Error {
    fn from(err: AssignmentError) -> Self {
        Self::Assignment(err)
    }
}

impl From<&'static str> for Error {
    fn from(message: &'static str) -> Self {
        Self::Other(message)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DecisionResult {
    Conflict,
    Sat,
}

impl DecisionResult {
    pub fn is_sat(&self) -> bool {
        matches!(self, Self::Sat)
    }
}

#[derive(Debug)]
pub enum SolveResult<'a> {
    Unsat,
    Sat(SatResult<'a>),
}

impl<'a> SolveResult<'a> {
    fn sat(model: &'a Model2) -> Self {
        Self::Sat(SatResult { model })
    }

    pub fn is_sat(&self) -> bool {
        matches!(self, SolveResult::Sat(_))
    }

    pub fn is_unsat(&self) -> bool {
        !self.is_sat()
    }
}

#[derive(Debug)]
pub struct SatResult<'a> {
    model: &'a Model2,
}

impl<'a> SatResult<'a> {
    pub fn model(&self) -> &'a Model2 {
        self.model
    }
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    len_variables: usize,
    clauses: ClauseDb,
    assignment2: Assignment2,
    decider: Decider,
    last_model2: LastModel2,
}

impl Solver {
    /// Returns the number of currently registered variables.
    fn len_variables(&self) -> usize {
        self.len_variables
    }

    pub fn from_cnf<I>(input: &mut I) -> Result<Self, CnfError<Error>>
    where
        I: Input,
    {
        let mut builder = SolverBuilder::default();
        cnf_parser::parse_cnf(input, &mut builder)?;
        Ok(builder.finalize())
    }

    /// Consumes the given clause.
    ///
    /// # Errors
    ///
    /// If the clause is unit and is in conflict with the current assignment.
    /// This is mostly encountered upon consuming two conflicting unit clauses.
    /// In this case the clause will not be added as new constraint.
    pub fn consume_clause(&mut self, clause: Clause) -> Result<(), Error> {
        // println!("Solver::consume_clause");
        match self.clauses.push_get(clause) {
            Ok(clause) => {
                // println!("Solver::consume_clause normal clause: {:?}", clause);
                self.assignment2.initialize_watchers(clause);
            }
            Err(unit_clause) => {
                // println!(
                //     "Solver::consume_clause unit clause: {:?}",
                //     unit_clause.literal
                // );
                self.assignment2
                    .enqueue_assumption(unit_clause.literal)
                    .map_err(|_| Error::Conflict)?;
            }
        }
        Ok(())
    }

    /// Returns the next variable.
    fn new_variable(&mut self) -> Variable {
        self.assignment2.register_new_variables(1);
        self.decider.register_new_variables(1);
        let next_id = self.len_variables();
        let variable =
            Variable::from_index(next_id).expect("registered too many variables");
        self.len_variables += 1;
        variable
    }

    /// Registers a new literal for the solver and returns it.
    ///
    /// The returned literal has positive polarity.
    ///
    /// # Errors
    ///
    /// If there are too many variables in use after this operation.
    pub fn new_literal(&mut self) -> Literal {
        self.new_variable().into_literal(VarAssignment::True)
    }

    /// Allocates the given amount of new literals for the solver and returns them.
    ///
    /// # Note
    ///
    /// The new literals are returned as a chunk which serves the purpose of
    /// efficiently accessing them.
    ///
    /// # Panics
    ///
    /// If there are too many variables in use after this operation.
    pub fn new_literal_chunk(&mut self, amount: usize) -> LiteralChunk {
        let old_len = self.len_variables();
        let new_len = self.len_variables() + amount;
        let chunk = LiteralChunk::new(old_len, new_len)
            .expect("encountered unexpected invalid literal chunk");
        self.assignment2.register_new_variables(amount);
        self.decider.register_new_variables(amount);
        self.len_variables += amount;
        chunk
    }

    fn solve_for_decision(&mut self, decision: Literal) -> Result<DecisionResult, Error> {
        match self.assignment2.enqueue_assumption(decision) {
            Err(AssignmentError::Conflict) => return Ok(DecisionResult::Conflict),
            Err(AssignmentError::AlreadyAssigned) => {
                panic!("decision heuristic unexpectedly proposed already assigned variable for propagation")
            }
            Err(_) => panic!("encountered unexpected or unknown enqueue error"),
            Ok(_) => (),
        }
        // println!(
        //     "Solver::solve_for_decision assignment = {:#?}",
        //     self.assignment2
        // );
        let propagation_result = self.assignment2.propagate(&mut self.clauses);
        println!(
            "Solver::solve_for_decision propagation_result = {:?}",
            propagation_result
        );
        match propagation_result {
            PropagationResult2::Conflict => Ok(DecisionResult::Conflict),
            PropagationResult2::Consistent => {
                let result = self.decide_and_propagate()?;
                Ok(result)
            }
        }
    }

    fn decide_and_propagate(&mut self) -> Result<DecisionResult, Error> {
        println!("\n\nSolver::decide_and_propagate");
        let next_variable = self
            .decider
            .next_unassigned(self.assignment2.variable_assignment());
        match next_variable {
            None => {
                println!("Solver::decide_and_propagate found solution!");
                self.last_model2
                    .update(self.assignment2.variable_assignment())
                    .expect("encountered unexpected indeterminate variable assignment");
                Ok(DecisionResult::Sat)
            }
            Some(unassigned_variable) => {
                println!(
                    "Solver::decide_and_propagate unassigned_variable = {:?}",
                    unassigned_variable
                );
                let level = self.assignment2.bump_decision_level();
                if self
                    .solve_for_decision(
                        unassigned_variable.into_literal(VarAssignment::True),
                    )?
                    .is_sat()
                    || self
                        .solve_for_decision(
                            unassigned_variable.into_literal(VarAssignment::False),
                        )?
                        .is_sat()
                {
                    println!("Solver::decide_and_propagate SAT");
                    Ok(DecisionResult::Sat)
                } else {
                    println!("Solver::decide_and_propagate found conflict!");
                    self.assignment2.pop_decision_level(level);
                    Ok(DecisionResult::Conflict)
                }
            }
        }
    }

    pub fn solve<L>(&mut self, assumptions: L) -> Result<SolveResult, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        println!("Solver::solve len_variables = {}", self.len_variables());
        // If the set of clauses contain the empty clause: UNSAT
        if self.len_variables() == 0 {
            return Ok(SolveResult::sat(self.last_model2.get()))
        }
        // Propagate in case the set of clauses contained unit clauses.
        // Bail out if the instance is already in conflict with itself.
        println!("Solver::solve propagate unit clauses of the problem instance");
        let _root_level = self.assignment2.bump_decision_level();
        if self.assignment2.propagate(&mut self.clauses).is_conflict() {
            return Ok(SolveResult::Unsat)
        }
        // Enqueue assumptions and propagate them afterwards.
        // Bail out if the provided assumptions are in conflict with the instance.
        println!("Solver::solve add given assumptions and propagate them");
        let _assumptions_level = self.assignment2.bump_decision_level();
        for assumption in assumptions {
            if let Err(AssignmentError::Conflict) =
                self.assignment2.enqueue_assumption(assumption)
            {
                return Ok(SolveResult::Unsat)
            }
        }
        if self.assignment2.propagate(&mut self.clauses).is_conflict() {
            return Ok(SolveResult::Unsat)
        }
        let _constraints_level = self.assignment2.bump_decision_level();
        println!("Solver::solve dive into decide and propagate iteration");
        // println!("Solver::solve assignment = {:#?}", self.assignment2);
        let result = match self.decide_and_propagate()? {
            DecisionResult::Conflict => SolveResult::Unsat,
            DecisionResult::Sat => {
                let result = SolveResult::sat(self.last_model2.get());
                println!("Solver::solve model = {}", self.last_model2.get());
                result
            }
        };
        // println!("Solver::solve assignment = {:#?}", self.assignment2);
        println!("Solver::solve new_result = {:#x?}", result);
        Ok(result)
    }
}
