#![forbid(unsafe_code)]
#![allow(clippy::len_without_is_empty)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod assignment;
mod builder;
mod clause_db;
mod decider;
mod literal;
mod literal_chunk;

#[cfg(test)]
mod tests;

use crate::{
    assignment::{
        Assignment,
        AssignmentError,
        LastModel,
        Model,
        PropagationResult,
    },
    builder::SolverBuilder,
    clause_db::{
        ClauseBuilder,
        ClauseDb,
    },
    decider::Decider,
};
pub use crate::{
    clause_db::ClauseDbError,
    literal::{
        Literal,
        Sign,
        Variable,
    },
    literal_chunk::{
        LiteralChunk,
        LiteralChunkIter,
    },
};
use alloc::vec::Vec;
use bounded::Bool;
use cnf_parser::{
    Error as CnfError,
    Input,
};
use core::{
    fmt,
    fmt::Display,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Other(&'static str),
    ClauseDb(ClauseDbError),
    Assignment(AssignmentError),
    Bounded(bounded::OutOfBoundsAccess),
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

impl From<bounded::OutOfBoundsAccess> for Error {
    fn from(err: bounded::OutOfBoundsAccess) -> Self {
        Self::Bounded(err)
    }
}

impl From<AssignmentError> for Error {
    fn from(err: AssignmentError) -> Self {
        Self::Assignment(err)
    }
}

impl From<ClauseDbError> for Error {
    fn from(err: ClauseDbError) -> Self {
        Self::ClauseDb(err)
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
    fn sat(model: &'a Model) -> Self {
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
    model: &'a Model,
}

impl<'a> Display for SatResult<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.model.fmt(f)
    }
}

impl<'a> SatResult<'a> {
    pub fn model(&self) -> &'a Model {
        self.model
    }
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    len_variables: usize,
    clause_builder: ClauseBuilder,
    clauses: ClauseDb,
    assignment: Assignment,
    decider: Decider,
    last_model2: LastModel,
    hard_facts: Vec<Literal>,
    encountered_empty_clause: bool,
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
    /// If the consumed clause is the empty clause.
    /// In this case the clause will be ignored.
    pub fn consume_clause<L>(&mut self, literals: L)
    where
        L: IntoIterator<Item = Literal>,
    {
        match self.clause_builder.build(literals) {
            Ok(verified_clause) => {
                let clause = self.clauses.push_clause(verified_clause);
                self.assignment.initialize_watchers(clause);
                for literal in clause {
                    let variable = literal.variable();
                    self.decider.bump_priority_by(variable, 1);
                }
            }
            Err(ClauseDbError::UnitClause { literal }) => {
                self.hard_facts.push(literal);
            }
            Err(ClauseDbError::TautologicClause) => (),
            Err(ClauseDbError::EmptyClause) => {
                self.encountered_empty_clause = true;
            },
        }
    }

    /// Returns the next variable.
    fn new_variable(&mut self) -> Variable {
        self.assignment.register_new_variables(1);
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
        self.new_variable().into_literal(Sign::True)
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
        self.assignment.register_new_variables(amount);
        self.decider.register_new_variables(amount);
        self.len_variables += amount;
        chunk
    }

    fn solve_for_decision(&mut self, decision: Literal) -> Result<DecisionResult, Error> {
        match self.assignment.enqueue_assumption(decision, None) {
            Err(AssignmentError::Conflict) => return Ok(DecisionResult::Conflict),
            Err(AssignmentError::AlreadyAssigned) => {
                panic!("decision heuristic unexpectedly proposed already assigned variable for propagation")
            }
            Err(_) => panic!("encountered unexpected or unknown enqueue error"),
            Ok(_) => (),
        }
        let propagation_result = self
            .assignment
            .propagate(&mut self.clauses, self.decider.informer());
        match propagation_result {
            PropagationResult::Conflict => Ok(DecisionResult::Conflict),
            PropagationResult::Consistent => {
                let result = self.decide_and_propagate()?;
                Ok(result)
            }
        }
    }

    fn decide_and_propagate(&mut self) -> Result<DecisionResult, Error> {
        let next_variable = self
            .decider
            .next_unassigned(self.assignment.variable_assignment());
        match next_variable {
            None => {
                self.last_model2
                    .update(self.assignment.variable_assignment())
                    .expect("encountered unexpected indeterminate variable assignment");
                Ok(DecisionResult::Sat)
            }
            Some(unassigned_variable) => {
                let level = self.assignment.bump_decision_level();
                if self
                    .solve_for_decision(unassigned_variable.into_literal(Sign::True))?
                    .is_sat()
                    || self
                        .solve_for_decision(
                            unassigned_variable.into_literal(Sign::False),
                        )?
                        .is_sat()
                {
                    Ok(DecisionResult::Sat)
                } else {
                    self.assignment
                        .pop_decision_level(level, self.decider.informer());
                    Ok(DecisionResult::Conflict)
                }
            }
        }
    }

    pub fn solve<L>(&mut self, assumptions: L) -> Result<SolveResult, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        // If the set of clauses contain the empty clause: UNSAT
        if self.encountered_empty_clause {
            return Ok(SolveResult::Unsat)
        }
        // IF the problem contains no variables the solution is trivial: SAT
        if self.len_variables() == 0 {
            return Ok(SolveResult::sat(self.last_model2.get()))
        }
        // Propagate known hard facts (unit clauses).
        for &hard_fact in &self.hard_facts {
            match self.assignment.enqueue_assumption(hard_fact, None) {
                Ok(()) => (),
                Err(AssignmentError::AlreadyAssigned) => (),
                Err(AssignmentError::Conflict) => return Ok(SolveResult::Unsat),
                _unexpected_error => {
                    panic!("encountered unexpected error while propagating hard facts")
                }
            }
        }
        if self
            .assignment
            .propagate(&mut self.clauses, self.decider.informer())
            .is_conflict()
        {
            return Ok(SolveResult::Unsat)
        }
        // Propagate in case the set of clauses contained unit clauses.
        // Bail out if the instance is already in conflict with itself.
        let _root_level = self.assignment.bump_decision_level();
        if self
            .assignment
            .propagate(&mut self.clauses, self.decider.informer())
            .is_conflict()
        {
            return Ok(SolveResult::Unsat)
        }
        // Enqueue assumptions and propagate them afterwards.
        // Bail out if the provided assumptions are in conflict with the instance.
        let _assumptions_level = self.assignment.bump_decision_level();
        for assumption in assumptions {
            if let Err(AssignmentError::Conflict) =
                self.assignment.enqueue_assumption(assumption, None)
            {
                return Ok(SolveResult::Unsat)
            }
        }
        if self
            .assignment
            .propagate(&mut self.clauses, self.decider.informer())
            .is_conflict()
        {
            return Ok(SolveResult::Unsat)
        }
        let _constraints_level = self.assignment.bump_decision_level();
        let result = match self.decide_and_propagate()? {
            DecisionResult::Conflict => SolveResult::Unsat,
            DecisionResult::Sat => {
                let result = SolveResult::sat(self.last_model2.get());
                result
            }
        };
        Ok(result)
    }
}
