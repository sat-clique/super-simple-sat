#![forbid(unsafe_code)]

mod assignment;
mod builder;
mod clause_db;
mod occurrence_map;
mod propagator;

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
        Literal,
        Model,
        VarAssignment,
        Variable,
    },
    clause_db::Clause,
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
    InvalidLiteralChunkRange,
    InvalidLiteralChunkStart,
    InvalidLiteralChunkEnd,
    TooManyVariablesInUse,
    InvalidDecisionId,
    InvalidDecisionStart,
    InvalidDecisionEnd,
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

impl From<&'static str> for Error {
    fn from(message: &'static str) -> Self {
        Self::Other(message)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SolveResult {
    Conflict,
    Sat,
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    len_variables: usize,
    clauses: ClauseDb,
    occurrence_map: OccurrenceMap,
    assignments: Assignment,
    propagator: Propagator,
}

/// A chunk of literals.
///
/// Created by the [`Solver::new_literal_chunk`] method.
#[derive(Debug, Clone)]
pub struct LiteralChunk {
    /// The start index of this chunk for the first literal.
    start_index: usize,
    /// The number of literals in the chunk.
    len: usize,
}

impl LiteralChunk {
    /// Creates a new literal chunk for the given start index and length.
    fn new(start_index: usize, end_index: usize) -> Result<Self, Error> {
        if start_index >= end_index {
            return Err(Error::InvalidLiteralChunkRange)
        }
        if !Variable::is_valid_index(start_index) {
            return Err(Error::InvalidLiteralChunkStart)
        }
        if !Variable::is_valid_index(end_index) {
            return Err(Error::InvalidLiteralChunkEnd)
        }
        Ok(Self {
            start_index,
            len: end_index - start_index,
        })
    }

    /// Returns the number of literals in this chunk.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the n-th literal of the chunk if within bounds.
    pub fn get(&self, n: usize) -> Option<Literal> {
        if n >= self.len() {
            return None
        }
        Some(
            Variable::from_index(self.start_index + n)
                .expect("encountered unexpected out of bounds variable index")
                .into_literal(VarAssignment::True),
        )
    }
}

impl IntoIterator for LiteralChunk {
    type Item = Literal;
    type IntoIter = LiteralChunkIter;

    fn into_iter(self) -> Self::IntoIter {
        LiteralChunkIter::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct LiteralChunkIter {
    current: usize,
    chunk: LiteralChunk,
}

impl LiteralChunkIter {
    fn new(chunk: LiteralChunk) -> Self {
        Self { current: 0, chunk }
    }
}

impl Iterator for LiteralChunkIter {
    type Item = Literal;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.chunk.len() - self.current;
        (remaining, Some(remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        match self.chunk.get(self.current) {
            None => None,
            Some(literal) => {
                self.current += 1;
                Some(literal)
            }
        }
    }
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

    fn solve_for(
        &mut self,
        current_var: Variable,
        assignment: VarAssignment,
    ) -> Result<SolveResult, Error> {
        self.assignments.assign(current_var, assignment)?;
        let current_lit = current_var.into_literal(assignment);
        match self.propagator.propagate(
            current_lit,
            &self.clauses,
            &self.occurrence_map,
            &mut self.assignments,
        )? {
            PropagationResult::Conflict { decision } => {
                self.propagator
                    .unassign_decision(decision, &mut self.assignments)?;
                Ok(SolveResult::Conflict)
            }
            PropagationResult::Consistent { decision } => {
                let next_var = self
                    .assignments
                    .next_unassigned(Some(current_var))
                    .expect("encountered unexpected invalid variable");
                let result = match next_var {
                    None => {
                        self.propagator.update_last_model(&self.assignments)?;
                        SolveResult::Sat
                    }
                    Some(unassigned_var) => {
                        if let SolveResult::Sat =
                            self.solve_for(unassigned_var, VarAssignment::True)?
                        {
                            SolveResult::Sat
                        } else if let SolveResult::Sat =
                            self.solve_for(unassigned_var, VarAssignment::False)?
                        {
                            SolveResult::Sat
                        } else {
                            SolveResult::Conflict
                        }
                    }
                };
                self.propagator
                    .unassign_decision(decision, &mut self.assignments)?;
                Ok(result)
            }
        }
    }

    pub fn solve<L>(&mut self, assumptions: L) -> Result<bool, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        // If the set of clauses contain the empty clause: UNSAT
        if self.len_variables() == 0 {
            return Ok(true)
        }
        for assumption in assumptions {
            let (variable, assignment) = assumption.into_var_and_assignment();
            self.assignments.assign(variable, assignment)?;
            if let PropagationResult::Conflict { decision: _ } =
                self.propagator.propagate(
                    assumption,
                    &self.clauses,
                    &self.occurrence_map,
                    &mut self.assignments,
                )?
            {
                return Ok(false)
            }
        }
        let initial_var = self
            .assignments
            .next_unassigned(None)
            .expect("encountered unexpected invalid initial variable");
        match initial_var {
            None => Ok(true),
            Some(initial_var) => {
                if let SolveResult::Sat =
                    self.solve_for(initial_var, VarAssignment::True)?
                {
                    return Ok(true)
                }
                if let SolveResult::Sat =
                    self.solve_for(initial_var, VarAssignment::False)?
                {
                    return Ok(true)
                }
                Ok(false)
            }
        }
    }

    pub(crate) fn last_model(&self) -> Option<&Model> {
        self.propagator.last_model()
    }

    pub fn print_last_model(&self) {
        if let Some(last_model) = self.last_model() {
            for (variable, assignment) in last_model {
                let index = variable.into_index();
                let assignment = assignment.to_bool().to_string();
                println!("Var: {:3}\t Value: {}", index, assignment);
            }
        } else {
            println!("no model found");
        }
    }
}
