mod assignment;
mod builder;
mod clause_db;
mod occurrence_map;

#[cfg(test)]
mod tests;

use crate::{
    assignment::Assignment,
    builder::SolverBuilder,
    clause_db::{
        ClauseDb,
        ClauseId,
    },
    occurrence_map::OccurrenceMap,
};
pub use crate::{
    assignment::{
        Literal,
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

#[derive(Debug, PartialEq, Eq)]
enum ClauseStatus {
    Conflicting,
    UndeterminedLiteral(Literal),
    NoConflictNorForcedAssignment,
}

#[derive(Debug, PartialEq, Eq)]
enum PropagationResult {
    Conflict { assigned: Vec<Literal> },
    Consistent { assigned: Vec<Literal> },
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
    last_model: Option<Assignment>,
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

    fn get_clause_status(&self, id: ClauseId) -> Option<ClauseStatus> {
        let mut num_indeterminate_lits = 0;
        let mut last_indeterminate_lit = None;
        for lit in self.clauses.resolve(id)? {
            match self
                .assignments
                .is_satisfied(lit)
                .expect("encountered unexpected invalid literal")
            {
                Some(true) => return Some(ClauseStatus::NoConflictNorForcedAssignment),
                Some(false) => {}
                None => {
                    last_indeterminate_lit = Some(lit);
                    num_indeterminate_lits += 1;
                }
            }
        }
        match num_indeterminate_lits {
            0 => Some(ClauseStatus::Conflicting),
            1 => {
                Some(ClauseStatus::UndeterminedLiteral(
                    last_indeterminate_lit
                        .expect("encountered missing expected undetermined literal"),
                ))
            }
            _ => Some(ClauseStatus::NoConflictNorForcedAssignment),
        }
    }

    fn propagate(&mut self, root_literal: Literal) -> Result<PropagationResult, Error> {
        let mut propagation_queue = vec![root_literal];
        let mut level_assignments = vec![root_literal];
        while let Some(lit_to_propagate) = propagation_queue.pop() {
            for possibly_false_clause_id in self
                .occurrence_map
                .iter_potentially_conflicting_clauses(lit_to_propagate)
            {
                match self
                    .get_clause_status(possibly_false_clause_id)
                    .expect("encountered invalid clause identifier")
                {
                    ClauseStatus::Conflicting => {
                        return Ok(PropagationResult::Conflict {
                            assigned: level_assignments,
                        })
                    }
                    ClauseStatus::UndeterminedLiteral(propagation_lit) => {
                        level_assignments.push(propagation_lit);
                        let (variable, var_assignment) =
                            propagation_lit.into_var_and_assignment();
                        self.assignments.assign(variable, var_assignment)?;
                        propagation_queue.push(propagation_lit);
                    }
                    _ => (),
                }
            }
        }
        Ok(PropagationResult::Consistent {
            assigned: level_assignments,
        })
    }

    fn undo_current_level_assignments<L>(
        &mut self,
        conflicting_lits: L,
    ) -> Result<(), Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        for conflicting_lit in conflicting_lits {
            self.assignments.unassign(conflicting_lit.variable())?;
        }
        Ok(())
    }

    fn solve_for(
        &mut self,
        current_var: Variable,
        assignment: VarAssignment,
    ) -> Result<SolveResult, Error> {
        self.assignments.assign(current_var, assignment)?;
        let current_lit = current_var.into_literal(assignment);
        match self.propagate(current_lit)? {
            PropagationResult::Conflict { assigned } => {
                self.undo_current_level_assignments(assigned)?;
                Ok(SolveResult::Conflict)
            }
            PropagationResult::Consistent { assigned } => {
                let next_var = self
                    .assignments
                    .next_unassigned(Some(current_var))
                    .expect("encountered unexpected invalid variable");
                let result = match next_var {
                    None => {
                        self.last_model = Some(self.assignments.clone());
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
                self.undo_current_level_assignments(assigned)?;
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
            if let PropagationResult::Conflict { assigned: _ } =
                self.propagate(assumption)?
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

    #[cfg(test)]
    pub(crate) fn last_model(&self) -> Option<&Assignment> {
        self.last_model.as_ref()
    }

    pub fn print_last_model(&self) {
        if let Some(last_model) = &self.last_model {
            for (variable, assignment) in last_model {
                let index = variable.into_index();
                let assignment = match assignment {
                    Some(assigned) => assigned.to_bool().to_string(),
                    None => "unassigned".to_string(),
                };
                println!("Var: {:3}\t Value: {}", index, assignment);
            }
        } else {
            println!("no model found");
        }
    }
}
