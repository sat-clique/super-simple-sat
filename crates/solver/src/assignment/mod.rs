mod model;
mod trail;
mod watch_list;

pub use self::model::{
    LastModel,
    Model,
    ModelIter,
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
    Bool,
    ClauseDatabase,
    Literal,
    RegisterVariables,
    Sign,
    Variable,
};
use bounded::{
    bounded_map,
    BoundedMap,
};
use core::{
    fmt::{
        self,
        Display,
    },
    ops::Not,
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
    /// When trying to assign a conflict.
    Conflict,
}

impl Display for AssignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndeterminateAssignment => {
                write!(f, "cannot create a model from an indeterminate assignment")
            }
            Self::InvalidVariable => write!(f, "the variable is invalid or unknown"),
            Self::AlreadyAssigned => write!(f, "the variable has already been assigned"),
            Self::Conflict => write!(f, "the assignment caused a conflict"),
        }
    }
}

impl AssignmentError {
    /// Returns `true` if the assignment error was caused by a conflict.
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
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

/// The partial variable assignment.
#[derive(Debug, Default, Clone)]
pub struct PartialAssignment {
    assignment: BoundedMap<Variable, Sign>,
}

impl RegisterVariables for PartialAssignment {
    fn register_variables(&mut self, additional: usize) {
        let new_len = self.len() + additional;
        self.assignment.resize_capacity(new_len);
    }
}

impl PartialAssignment {
    /// Returns the number of registered variables.
    pub fn len(&self) -> usize {
        self.assignment.capacity()
    }

    /// Returns the number of assigned variables.
    pub fn len_assigned(&self) -> usize {
        self.assignment.len()
    }

    /// Returns `true` if the partial assignment is complete.
    pub fn is_complete(&self) -> bool {
        self.len_assigned() == self.len()
    }

    /// Returns an iterator yielding shared references to the variable assignments.
    ///
    /// # Note
    ///
    /// Variables that have not been assigned, yet will not be yielded.
    pub fn iter(&self) -> bounded_map::Iter<Variable, Sign> {
        self.assignment.iter()
    }

    /// Returns the assignment for the given variable.
    ///
    /// # Panics
    ///
    /// If the variable is invalid and cannot be resolved.
    pub fn get(&self, variable: Variable) -> Option<Sign> {
        self.assignment
            .get(variable)
            .expect("encountered unexpected invalid variable")
            .copied()
    }

    /// Returns `true` if the given literal is satisfied under the current assignment.
    ///
    /// Returns `None` if the assignment is indeterminate.
    ///
    /// # Panics
    ///
    /// If the variable is invalid and cannot be resolved.
    pub fn is_satisfied(&self, literal: Literal) -> Option<bool> {
        self.get(literal.variable())
            .map(Sign::into_bool)
            .map(|assignment| literal.sign().into_bool() == assignment)
    }

    /// Returns `true` if the given literal is conflicting with the current assignment.
    ///
    /// Returns `None` if the assignment is indeterminate.
    ///
    /// # Panics
    ///
    /// If the variable is invalid and cannot be resolved.
    pub fn is_conflicting(&self, literal: Literal) -> Option<bool> {
        self.is_satisfied(literal).map(Not::not)
    }

    /// Updates the assignment of the variable.
    ///
    /// # Panics
    ///
    /// - If the variable is invalid and cannot be resolved.
    /// - If the variable has already been assigned.
    pub fn assign(&mut self, variable: Variable, assignment: Sign) {
        let old_assignment = self
            .assignment
            .insert(variable, assignment)
            .expect("encountered unexpected invalid variable");
        assert!(old_assignment.is_none());
    }

    /// Unassigns the given variable assignment.
    ///
    /// # Panics
    ///
    /// - If the variable is invalid and cannot be resolved.
    /// - If the variable has already been unassigned.
    pub fn unassign(&mut self, variable: Variable) {
        let old_assignment = self
            .assignment
            .take(variable)
            .expect("encountered unexpected invalid variable");
        assert!(old_assignment.is_some());
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

impl<'a> IntoIterator for &'a PartialAssignment {
    type Item = (Variable, Sign);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a> IntoIterator for &'a Assignment {
    type Item = (Variable, Sign);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(&self.assignments)
    }
}

pub struct Iter<'a> {
    iter: bounded_map::Iter<'a, Variable, Sign>,
}

impl<'a> Iter<'a> {
    pub fn new(assignment: &'a PartialAssignment) -> Self {
        Self {
            iter: assignment.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Variable, Sign);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(variable, assignment)| (variable, *assignment))
    }
}
