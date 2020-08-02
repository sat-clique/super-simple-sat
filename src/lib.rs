#![forbid(unsafe_code)]
#![allow(clippy::len_without_is_empty)]

mod assignment;
mod assignment2;
mod builder;
mod clause_db;
mod literal;
mod literal_chunk;
mod occurrence_map;
mod propagator;
mod utils;

#[cfg(test)]
mod tests;

use crate::{
    assignment::Assignment,
    builder::SolverBuilder,
    clause_db::ClauseDb,
    occurrence_map::OccurrenceMap,
    propagator::{
        PropagationResult,
        Propagator,
    },
};
pub use crate::{
    assignment::{
        LastModel,
        Model,
    },
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
    Occurrences(occurrence_map::Error),
    Assignment(assignment::Error),
    Bounded(utils::Error),
    InvalidLiteralChunkRange,
    InvalidLiteralChunkStart,
    InvalidLiteralChunkEnd,
    TooManyVariablesInUse,
    InvalidDecisionId,
    InvalidDecisionStart,
    InvalidDecisionEnd,
    InvalidSizeIncrement,
}

impl From<occurrence_map::Error> for Error {
    fn from(err: occurrence_map::Error) -> Self {
        Self::Occurrences(err)
    }
}

impl From<assignment::Error> for Error {
    fn from(err: assignment::Error) -> Self {
        Self::Assignment(err)
    }
}

impl From<utils::Error> for Error {
    fn from(err: utils::Error) -> Self {
        Self::Bounded(err)
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

pub struct SatResult<'a> {
    model: &'a Model,
}

impl<'a> SatResult<'a> {
    pub fn model(&self) -> &'a Model {
        self.model
    }
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    len_variables: usize,
    clauses: ClauseDb,
    occurrence_map: OccurrenceMap,
    assignments: Assignment,
    propagator: Propagator,
    last_model: LastModel,
}

impl Solver {
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

    pub fn consume_clause(&mut self, clause: Clause) -> Result<(), Error> {
        let id = self.clauses.push(clause);
        for literal in self
            .clauses
            .resolve(id)
            .expect("unexpected missing clause that has just been inserted")
        {
            self.occurrence_map.register_for_lit(literal, id)?
        }
        Ok(())
    }

    /// Allocates a new literal for the solver and returns it.
    ///
    /// # Errors
    ///
    /// If there are too many variables in use after this operation.
    pub fn new_literal(&mut self) -> Result<Literal, Error> {
        self.occurrence_map.register_variables(1)?;
        self.assignments.register_variables(1)?;
        let next_id = self.len_variables();
        let variable =
            Variable::from_index(next_id).ok_or_else(|| Error::TooManyVariablesInUse)?;
        self.len_variables += 1;
        Ok(variable.into_literal(VarAssignment::True))
    }

    /// Allocates the given amount of new literals for the solver and returns them.
    ///
    /// # Note
    ///
    /// The new literals are returned as a chunk which serves the purpose of
    /// efficiently accessing them.
    ///
    /// # Errors
    ///
    /// If there are too many variables in use after this operation.
    pub fn new_literal_chunk(&mut self, amount: usize) -> Result<LiteralChunk, Error> {
        let old_len = self.len_variables();
        let new_len = self.len_variables() + amount;
        let chunk = LiteralChunk::new(old_len, new_len)?;
        self.occurrence_map.register_variables(amount)?;
        self.assignments.register_variables(amount)?;
        self.len_variables += amount;
        Ok(chunk)
    }

    fn solve_for_decision(
        &mut self,
        current_var: Variable,
        assignment: VarAssignment,
    ) -> Result<DecisionResult, Error> {
        let current_lit = current_var.into_literal(assignment);
        match self.propagator.propagate(
            current_lit,
            &self.clauses,
            &self.occurrence_map,
            &mut self.assignments,
        )? {
            PropagationResult::Conflict { decision } => {
                self.propagator
                    .backtrack_decision(decision, &mut self.assignments)?;
                Ok(DecisionResult::Conflict)
            }
            PropagationResult::Consistent { decision } => {
                let result = self.solve_for_next_unassigned(Some(current_var))?;
                self.propagator
                    .backtrack_decision(decision, &mut self.assignments)?;
                Ok(result)
            }
        }
    }

    fn solve_for_next_unassigned(
        &mut self,
        current_variable: Option<Variable>,
    ) -> Result<DecisionResult, Error> {
        let next_var = self
            .assignments
            .next_unassigned(current_variable)
            .expect("encountered unexpected invalid variable");
        match next_var {
            None => {
                self.last_model.update(&self.assignments)?;
                Ok(DecisionResult::Sat)
            }
            Some(unassigned_var) => {
                let (len_pos, len_neg) =
                    self.occurrence_map.len_pos_neg(unassigned_var)?;
                let prediction = VarAssignment::from_bool(len_pos >= len_neg);
                if self
                    .solve_for_decision(unassigned_var, prediction)?
                    .is_sat()
                    || self
                        .solve_for_decision(unassigned_var, !prediction)?
                        .is_sat()
                {
                    Ok(DecisionResult::Sat)
                } else {
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
        if self.len_variables() == 0 {
            return Ok(SolveResult::sat(self.last_model.get()))
        }
        for assumption in assumptions {
            if let PropagationResult::Conflict { decision: _ } =
                self.propagator.propagate(
                    assumption,
                    &self.clauses,
                    &self.occurrence_map,
                    &mut self.assignments,
                )?
            {
                return Ok(SolveResult::Unsat)
            }
        }
        match self.solve_for_next_unassigned(None)? {
            DecisionResult::Conflict => Ok(SolveResult::Unsat),
            DecisionResult::Sat => Ok(SolveResult::sat(self.last_model.get())),
        }
    }
}
