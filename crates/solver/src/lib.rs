#![deny(unsafe_code)]
#![warn(unsafe_op_in_unsafe_fn)]
#![allow(clippy::len_without_is_empty)]

mod assignment;
mod builder;
pub mod clause_db;
mod decider;
mod literal;
mod literal_chunk;
mod sanitizer;

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
    clause_db::ClauseDatabase,
    decider::Decider,
    literal::RegisterVariables,
    sanitizer::{
        ClauseSanitizer,
        SanitizedLiterals,
    },
};
pub use crate::{
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
use bounded::{
    Bool,
    Index as _,
};
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
    Assignment(AssignmentError),
    Bounded(bounded::OutOfBoundsAccess),
    Conflict,
    InvalidLiteralChunk,
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

impl From<&'static str> for Error {
    fn from(message: &'static str) -> Self {
        Self::Other(message)
    }
}

/// The result of a decision followed by propagation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DecisionResult {
    /// The decision caused a conflict.
    Conflict,
    /// The decision resulted in a satisfying assignment.
    Sat,
}

impl DecisionResult {
    /// Returns `true` if the decision result yielded a satisfying assignment.
    pub fn is_sat(&self) -> bool {
        matches!(self, Self::Sat)
    }
}

/// The satisfiable or unsatisfiable solution to a SAT instance.
///
/// # Note
///
/// If the solution is satisfiable it also contains a satisfying assignment.
#[derive(Debug)]
pub enum SolveResult<'a> {
    /// The SAT instance is unsatisfiable.
    Unsat,
    /// The SAT instance is satisfiable with the given satisfying assignment.
    Sat(SatResult<'a>),
}

impl<'a> SolveResult<'a> {
    fn sat(model: &'a Model) -> Self {
        Self::Sat(SatResult { model })
    }

    /// Returns `true` if the SAT instance was determined to be satisfiable.
    pub fn is_sat(&self) -> bool {
        matches!(self, SolveResult::Sat(_))
    }

    /// Returns `true` if the SAT instance was determined to be unsatisfiable.
    pub fn is_unsat(&self) -> bool {
        !self.is_sat()
    }
}

/// The satisfiable solution of a solved SAT instance.
#[derive(Debug)]
pub struct SatResult<'a> {
    /// The underlying complete variable assignment.
    model: &'a Model,
}

impl<'a> Display for SatResult<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.model.fmt(f)
    }
}

impl<'a> SatResult<'a> {
    /// The satisfying assignment of the satisfiable solution.
    pub fn model(&self) -> &'a Model {
        self.model
    }
}

/// The solver instance.
#[derive(Debug, Default, Clone)]
pub struct Solver {
    /// The number of registered variables.
    len_variables: usize,
    /// The clause database that stores all information about clauses.
    clauses: ClauseDatabase,
    /// The partial assignment of variables.
    assignment: Assignment,
    /// The decision heuristic.
    decider: Decider,
    /// The last full assignment found by the solver upon SAT.
    last_model: LastModel,
    /// Sanitizes clauses before being fed to the solver.
    sanitizer: ClauseSanitizer,
    /// Yields `true` if `consume_clause` encountered the empty clause.
    encountered_empty_clause: bool,
    /// Unit clauses that have been fed to `consume_clause`.
    ///
    /// They are immediately propagated when calling `solve`.
    hard_facts: Vec<Literal>,
}

impl RegisterVariables for Solver {
    fn register_variables(&mut self, additional: usize) {
        self.assignment.register_variables(additional);
        self.decider.register_variables(additional);
        self.sanitizer.register_variables(additional);
        self.len_variables += additional;
    }
}

impl Solver {
    /// Returns the number of currently registered variables.
    fn len_variables(&self) -> usize {
        self.len_variables
    }

    /// Consumes the `.cnf` input and feeds it to the returned solver.
    ///
    /// # Errors
    ///
    /// - If the input is no valid `.cnf` format.
    /// - If the input encodes an invalid CNF formula.
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
    /// # Panics
    ///
    /// If any of the literals has not been registered.
    pub fn consume_clause<I>(&mut self, literals: I)
    where
        I: IntoIterator,
        I::IntoIter: ExactSizeIterator<Item = Literal>,
    {
        match self.sanitizer.sanitize(literals) {
            SanitizedLiterals::Literals(literals) => {
                let cref = self.clauses.alloc(literals);
                let resolved = self.clauses.resolve(cref).unwrap_or_else(|| {
                    panic!("failed to resolve recently allocated clause: {:?}", cref)
                });
                self.assignment.initialize_watchers(cref, resolved);
                for literal in resolved.literals() {
                    let variable = literal.variable();
                    self.decider.bump_priority_by(variable, 1);
                }
            }
            SanitizedLiterals::UnitClause(unit) => {
                self.hard_facts.push(unit);
            }
            SanitizedLiterals::TautologicalClause => (),
            SanitizedLiterals::EmptyClause => {
                self.encountered_empty_clause = true;
            }
            SanitizedLiterals::InvalidLiteral(invalid_literal) => {
                panic!("encountered invalid literal: {}", invalid_literal)
            }
        }
    }

    /// Returns the next variable.
    fn new_variable(&mut self) -> Variable {
        let next_id = self.len_variables();
        self.register_variables(1);
        Variable::from_index(next_id)
    }

    /// Registers a new literal for the solver and returns it.
    ///
    /// # Note
    ///
    /// The returned literal has positive polarity.
    ///
    /// # Panics
    ///
    /// If more variables have been registered than supported by the solver limits.
    pub fn new_literal(&mut self) -> Literal {
        Literal::new(self.new_variable(), Sign::POS)
    }

    /// Allocates the given amount of new literals for the solver and returns them.
    ///
    /// # Note
    ///
    /// - The returned literals have positive polarity.
    /// - The returned literal chunk acts as an efficient iterator over the new literals.
    ///
    /// # Panics
    ///
    /// If more variables have been registered than supported by the solver limits.
    pub fn new_literal_chunk(&mut self, amount: usize) -> LiteralChunk {
        let first_index = self.len_variables();
        let chunk = LiteralChunk::new(first_index, amount).unwrap_or_else(|_| {
            panic!(
                "created invalid literal chunk for range ({}..{})",
                first_index,
                first_index + amount
            )
        });
        self.register_variables(amount);
        chunk
    }

    /// Starts solving the given SAT instance.
    pub fn solve<L>(&mut self, assumptions: L) -> Result<SolveResult, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        // If the set of clauses contain the empty clause: UNSAT
        if self.encountered_empty_clause {
            return Ok(SolveResult::Unsat)
        }

        // If the set of clauses contain the empty clause: UNSAT
        if self.len_variables() == 0 {
            return Ok(SolveResult::sat(self.last_model.get()))
        }

        // Raise decision level before propagating the hard problem facts.
        let _root_level = self.assignment.bump_decision_level();

        // Propagate known hard facts (unit clauses).
        if self.propagate_hard_facts().is_conflict() {
            return Ok(SolveResult::Unsat)
        }

        // Raise decision level before propagating the given assumptions.
        let _assumptions_level = self.assignment.bump_decision_level();

        // Enqueue and propagate given assumptions.
        //
        // Bail out if the provided assumptions are in conflict with the instance.
        if self.propagate_assumptions(assumptions).is_conflict() {
            return Ok(SolveResult::Unsat)
        }

        // Raise decision level before propagating the decisions.
        let _constraints_level = self.assignment.bump_decision_level();

        // Start solving using recursive DPLL style.
        let result = match self.decide_and_propagate() {
            DecisionResult::Conflict => SolveResult::Unsat,
            DecisionResult::Sat => {
                let result = SolveResult::sat(self.last_model.get());
                result
            }
        };
        Ok(result)
    }

    /// Propagates the hard facts (unit clauses) of the SAT instance.
    fn propagate_hard_facts(&mut self) -> PropagationResult {
        for &hard_fact in &self.hard_facts {
            match self.assignment.enqueue_assumption(hard_fact) {
                Ok(()) | Err(AssignmentError::AlreadyAssigned) => (),
                Err(AssignmentError::ConflictingAssignment) => {
                    return PropagationResult::Conflict
                }
                _unexpected_error => {
                    panic!("encountered unexpected error while propagating hard facts")
                }
            }
        }
        PropagationResult::Consistent
    }

    /// Propagates the given assumptions.
    fn propagate_assumptions<L>(&mut self, assumptions: L) -> PropagationResult
    where
        L: IntoIterator<Item = Literal>,
    {
        for assumption in assumptions {
            if let Err(AssignmentError::ConflictingAssignment) =
                self.assignment.enqueue_assumption(assumption)
            {
                return PropagationResult::Conflict
            }
        }
        if self
            .assignment
            .propagate(&mut self.clauses, &mut self.decider)
            .is_conflict()
        {
            return PropagationResult::Conflict
        }
        PropagationResult::Consistent
    }

    /// Decides the next literal and propagates it.
    ///
    /// This recursively checks for a valid assignment for both
    /// positive and negative assignments of the decided literal.
    /// Returns a conflict if both assignments has led to a conflict.
    ///
    /// # Note
    ///
    /// Returns SAT if all literals already are assigned OR
    /// if a valid assignment has been found.
    fn decide_and_propagate(&mut self) -> DecisionResult {
        let next_variable = self
            .decider
            .next_unassigned(self.assignment.variable_assignment());
        match next_variable {
            None => {
                self.last_model
                    .update(self.assignment.variable_assignment())
                    .expect("encountered unexpected indeterminate variable assignment");
                DecisionResult::Sat
            }
            Some(unassigned_variable) => {
                let level = self.assignment.bump_decision_level();
                let decision = Literal::new(unassigned_variable, Sign::POS);
                if self.solve_for_decision(decision).is_sat()
                    || self.solve_for_decision(!decision).is_sat()
                {
                    return DecisionResult::Sat
                }
                self.assignment.pop_decision_level(level, &mut self.decider);
                DecisionResult::Conflict
            }
        }
    }

    /// Tries to find a valid assignment for the given literal decision.
    fn solve_for_decision(&mut self, decision: Literal) -> DecisionResult {
        match self.assignment.enqueue_assumption(decision) {
            Err(AssignmentError::ConflictingAssignment) => {
                return DecisionResult::Conflict
            }
            Err(AssignmentError::AlreadyAssigned) => {
                panic!(
                    "decision heuristic proposed already assigned variable for propagation: {:?}",
                    decision,
                )
            }
            Err(error) => {
                panic!("encountered unexpected or unknown enqueue error: {}", error)
            }
            Ok(_) => (),
        }
        let propagation_result = self
            .assignment
            .propagate(&mut self.clauses, &mut self.decider);
        match propagation_result {
            PropagationResult::Conflict => DecisionResult::Conflict,
            PropagationResult::Consistent => self.decide_and_propagate(),
        }
    }
}
