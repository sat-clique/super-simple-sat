mod model;
mod partial;
mod trail;
mod watch_list;

pub use self::{
    model::{
        LastModel,
        Model,
        ModelIter,
    },
    partial::PartialAssignment,
};
use self::{
    trail::{
        DecisionLevel,
        Trail,
    },
    watch_list::WatchList,
};
use crate::{
    clause_db::{
        ClauseRef,
        ResolvedClause,
    },
    decider::InformDecider,
    ClauseDatabase,
    Literal,
    RegisterVariables,
    Sign,
    Variable,
};
use bounded::bounded_map;
use core::fmt::{
    self,
    Display,
};

/// Errors that may be encountered when operating on the assignment.
#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentError {
    /// When trying to create a model from an indeterminate assignment.
    IndeterminateAssignment,
    /// Variable invalid for the current assignment.
    InvalidVariable,
    /// When trying to assign a variable that has already been assigned.
    AlreadyAssigned,
    /// When trying to make a conflicting assignment.
    ConflictingAssignment,
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndeterminateAssignment => {
                write!(f, "indeterminate assignment found when complete assignment was expected")
            }
            Self::InvalidVariable => {
                write!(f, "the variable for the assignment is invalid")
            }
            Self::AlreadyAssigned => write!(f, "the variable has already been assigned"),
            Self::ConflictingAssignment => {
                write!(f, "the assignment is in conflict with existing assignment")
            }
        }
    }
}

/// Allows to enqueue new literals into the propagation queue.
#[derive(Debug)]
pub struct PropagationEnqueuer<'a> {
    queue: &'a mut Trail,
}

impl<'a> PropagationEnqueuer<'a> {
    /// Returns a new wrapper around the given propagation queue.
    fn new(queue: &'a mut Trail) -> Self {
        Self { queue }
    }

    /// Enqueues a new literal to the propagation queue.
    ///
    /// # Errors
    ///
    /// - If the literal has already been satisfied.
    /// - If the literal is in conflict with the current assignment. This will
    ///   also clear the propagation queue.
    pub fn push(
        &mut self,
        literal: Literal,
        assignment: &mut PartialAssignment,
    ) -> Result<(), AssignmentError> {
        self.queue.push(literal, assignment)
    }
}

/// The database combining everything that is realted to variable assignment.
///
/// This holds and organizes data flows through:
///
/// - Variable assignment
/// - Decision trail
/// - 2-watched literals
/// - Propagation queue
#[derive(Debug, Default, Clone)]
pub struct Assignment {
    trail: Trail,
    assignments: PartialAssignment,
    watchers: WatchList,
}

impl RegisterVariables for Assignment {
    fn register_variables(&mut self, additional: usize) {
        self.trail.register_variables(additional);
        self.assignments.register_variables(additional);
        self.watchers.register_variables(additional);
    }
}

impl Assignment {
    /// Initializes the watchers of the assignment given the clause database.
    ///
    /// # Errors
    ///
    /// If the initialization has already taken place.
    pub fn initialize_watchers(&mut self, cref: ClauseRef, resolved: ResolvedClause) {
        let fst = *resolved.literals().first();
        let snd = *resolved.literals().second();
        self.watchers.register_for_lit(!fst, snd, cref);
        self.watchers.register_for_lit(!snd, fst, cref);
    }

    /// Returns a view into the assignment.
    pub fn variable_assignment(&self) -> &PartialAssignment {
        &self.assignments
    }

    /// Resets the assignment to the given decision level.
    pub fn reset_to_level(
        &mut self,
        level: DecisionLevel,
        inform_decider: InformDecider,
    ) {
        self.trail
            .pop_to_level(level, &mut self.assignments, inform_decider)
    }

    /// Enqueues a propagation literal.
    ///
    /// This does not yet perform the actual unit propagation.
    ///
    /// # Errors
    ///
    /// - If the pushed literal is in conflict with the current assignment.
    /// - If the literal has already been assigned.
    pub fn enqueue_assumption(
        &mut self,
        assumption: Literal,
    ) -> Result<(), AssignmentError> {
        self.trail.push(assumption, &mut self.assignments)
    }
}

/// The result of a propagation after a decision has been made.
#[derive(Debug, Copy, Clone)]
pub enum PropagationResult {
    /// Propagation led to a consistent assignment.
    Consistent,
    /// Propagation led to a conflicting assignment.
    Conflict,
}

impl PropagationResult {
    /// Returns `true` if the propagation yielded a conflict.
    pub fn is_conflict(self) -> bool {
        matches!(self, Self::Conflict)
    }
}

impl Assignment {
    /// Bumps the decision level.
    pub fn bump_decision_level(&mut self) -> DecisionLevel {
        self.trail.bump_decision_level()
    }

    /// Pops the decision level to the given level.
    ///
    /// This also unassigned all variables assigned in the given decision level.
    pub fn pop_decision_level(
        &mut self,
        level: DecisionLevel,
        inform_decider: InformDecider,
    ) {
        self.trail
            .pop_to_level(level, &mut self.assignments, inform_decider)
    }

    /// Propagates the enqueued assumptions.
    pub fn propagate(
        &mut self,
        clause_db: &mut ClauseDatabase,
        inform_decider: InformDecider,
    ) -> PropagationResult {
        let Self {
            watchers,
            assignments,
            trail,
            ..
        } = self;
        let level = trail.current_decision_level();
        while let Some(propagation_literal) = trail.pop_enqueued() {
            let result = watchers.propagate(
                propagation_literal,
                clause_db,
                assignments,
                PropagationEnqueuer::new(trail),
            );
            if result.is_conflict() {
                trail.pop_to_level(level, assignments, inform_decider);
                return result
            }
        }
        PropagationResult::Consistent
    }
}

impl<'a> IntoIterator for &'a Assignment {
    type Item = Literal;
    type IntoIter = AssignmentIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AssignmentIter::new(&self.assignments)
    }
}

/// Iterator over the assignments of all variables.
///
/// # Note
///
/// This effectively returns every literal in order.
/// The literal has positive polarity if the variable
/// has been assigned to true and otherwise negative
/// polarity.
pub struct AssignmentIter<'a> {
    iter: bounded_map::Iter<'a, Variable, Sign>,
}

impl<'a> AssignmentIter<'a> {
    pub fn new(assignment: &'a PartialAssignment) -> Self {
        Self {
            iter: assignment.iter(),
        }
    }
}

impl<'a> Iterator for AssignmentIter<'a> {
    type Item = Literal;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(variable, assignment)| Literal::new(variable, *assignment))
    }
}
