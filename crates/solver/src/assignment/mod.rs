mod first_uip_learning;
mod model;
mod trail;
mod watch_list;

pub use self::model::{
    LastModel,
    Model,
    ModelIter,
};
use self::{
    first_uip_learning::{
        FirstUipLearning,
        LearnedClauseLiterals,
    },
    trail::{
        DecisionLevel,
        Trail,
    },
    watch_list::WatchList,
};
use crate::{
    clause_db::{
        ClauseId,
        ClauseRef,
    },
    decider::InformDecider,
    Bool,
    ClauseDb,
    Literal,
    Sign,
    Variable,
};
use bounded::{
    bounded_map,
    BoundedMap,
};
use core::ops::Not;

/// Errors that may be encountered when operating on the assignment.
#[derive(Debug, PartialEq, Eq)]
pub enum AssignmentError {
    /// When trying to create a model from an indeterminate assignment.
    UnexpectedIndeterminateAssignment,
    /// Variable invalid for the current assignment.
    InvalidVariable,
    /// When trying to assign a variable that has already been assigned.
    AlreadyAssigned,
    /// When trying to assign a conflict.
    Conflict,
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
        reason: Option<ClauseId>,
        assignment: &mut VariableAssignment,
        level_and_decision: &mut DecisionLevelsAndReasons,
    ) -> Result<(), AssignmentError> {
        self.queue
            .push(literal, reason, assignment, level_and_decision)
    }
}

/// The actual variable assignment.
#[derive(Debug, Default, Clone)]
pub struct VariableAssignment {
    assignment: BoundedMap<Variable, Sign>,
}

impl VariableAssignment {
    /// Returns the number of registered variables.
    pub fn len(&self) -> usize {
        self.assignment.capacity()
    }

    /// Returns the number of assigned variables.
    pub fn len_assigned(&self) -> usize {
        self.assignment.len()
    }

    /// Returns `true` if the assignment is complete.
    pub fn is_complete(&self) -> bool {
        self.len() == self.len_assigned()
    }

    /// Returns an iterator yielding shared references to the variable assignments.
    ///
    /// # Note
    ///
    /// Variables that have not been assigned, yet will not be yielded.
    pub fn iter(&self) -> bounded_map::Iter<Variable, Sign> {
        self.assignment.iter()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Errors
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let new_len = self.len() + new_variables;
        self.assignment.resize_capacity(new_len);
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
            .map(|assignment| literal.assignment().into_bool() == assignment)
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

/// Decision level of a variable assignment and its reason if any.
#[derive(Debug, Copy, Clone)]
pub struct DecisionLevelAndReason {
    /// The decision the variable was last assigned on.
    level: DecisionLevel,
    /// The reason the variable was last inferred from (by unit propagation) if any.
    reason: Option<ClauseId>,
}

#[derive(Debug, Default, Clone)]
pub struct DecisionLevelsAndReasons {
    level_and_reason: BoundedMap<Variable, DecisionLevelAndReason>,
}

impl DecisionLevelsAndReasons {
    /// Registers the given number of additional variables.
    ///
    /// # Panics
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let total_capacity = self.level_and_reason.capacity() + new_variables;
        self.level_and_reason.resize_capacity(total_capacity);
    }

    /// Updates the decision level and reason clause ID for the given variable.
    ///
    /// # Panics
    ///
    /// If the given variable ID is out of bounds.
    pub fn update(
        &mut self,
        variable: Variable,
        level: DecisionLevel,
        reason: Option<ClauseId>,
    ) {
        self.level_and_reason
            .insert(variable, DecisionLevelAndReason { level, reason })
            .expect("encountered unexpected invalid variable");
    }

    /// Returns the decision level and reason tuple for the given variable if any.
    ///
    /// # Panics
    ///
    /// If the given variable ID is out of bounds.
    fn get(&self, variable: Variable) -> Option<(DecisionLevel, Option<ClauseId>)> {
        self.level_and_reason
            .get(variable)
            .expect("encountered unexpected invalid variable")
            .map(|level_and_reason| (level_and_reason.level, level_and_reason.reason))
    }

    /// Returns the reason clause ID of the given variable if any.
    ///
    /// # Note
    ///
    /// This returns `None` if the variable has never been assigned or is unassigned
    /// or if it is assigned by the trail but has no reason clause. Users can differentiate
    /// between both states using the [`get_level`] which only returns `None` if the
    /// variable has never been assigned.
    ///
    /// # Panics
    ///
    /// If the given variable ID is out of bounds.
    pub fn get_reason(&self, variable: Variable) -> Option<ClauseId> {
        self.get(variable).map(|(_, reason)| reason).flatten()
    }

    /// Returns the decision level of the given variable if it has been assigned already.
    ///
    /// # Panics
    ///
    /// If the given variable ID is out of bounds.
    pub fn get_level(&self, variable: Variable) -> Option<DecisionLevel> {
        self.get(variable).map(|(level, _)| level)
    }

    /// Returns `true` if the given variable assignment was forced by the trail.
    ///
    /// # Panics
    ///
    /// If the given variable ID is out of bounds.
    pub fn is_forced(&self, variable: Variable) -> bool {
        self.get(variable)
            .map(|(_, reason)| reason.is_some())
            .unwrap_or_else(|| false)
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
    assignments: VariableAssignment,
    watchers: WatchList,
    level_and_reason: DecisionLevelsAndReasons,
    first_uip_learning: FirstUipLearning,
}

impl Assignment {
    /// Initializes the watchers of the assignment given the clause database.
    ///
    /// # Errors
    ///
    /// If the initialization has already taken place.
    pub fn initialize_watchers(&mut self, clause: ClauseRef) {
        let clause_id = clause.id();
        let fst = clause.first();
        let snd = clause.second();
        self.watchers.register_for_lit(!fst, snd, clause_id);
        self.watchers.register_for_lit(!snd, fst, clause_id);
    }

    /// Returns a view into the assignment.
    pub fn variable_assignment(&self) -> &VariableAssignment {
        &self.assignments
    }

    /// Registers the given number of additional variables.
    ///
    /// # Panics
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        self.trail.register_new_variables(new_variables);
        self.assignments.register_new_variables(new_variables);
        self.watchers.register_new_variables(new_variables);
        self.level_and_reason.register_new_variables(new_variables);
        self.first_uip_learning
            .register_new_variables(new_variables);
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
        self.trail.push(
            assumption,
            None,
            &mut self.assignments,
            &mut self.level_and_reason,
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PropagationResult {
    /// Propagation led to a consistent assignment.
    Consistent,
    /// Propagation led to a conflicting assignment with the given conflicting clause.
    Conflict(ClauseId),
}

impl PropagationResult {
    /// Returns `true` if the propagation yielded a conflict.
    pub fn is_conflict(self) -> bool {
        matches!(self, Self::Conflict(_))
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
        clause_db: &mut ClauseDb,
        inform_decider: InformDecider,
    ) -> PropagationResult {
        let Self {
            watchers,
            assignments,
            level_and_reason,
            trail,
            first_uip_learning,
        } = self;
        let level = trail.current_decision_level();
        while let Some(propagation_literal) = trail.pop_enqueued() {
            let result = watchers.propagate(
                propagation_literal,
                clause_db,
                assignments,
                level_and_reason,
                PropagationEnqueuer::new(trail),
            );
            if let PropagationResult::Conflict(conflicting_clause) = result {
                #[cfg(test)]
                {
                    let conflicting_clause =
                        clause_db.resolve(conflicting_clause).expect(
                            "could not resolve conflicting clause in clause database",
                        );
                    let learned_clause = first_uip_learning.compute_conflict_clause(
                        conflicting_clause,
                        trail,
                        level_and_reason,
                        clause_db,
                    );
                    println!(
                        "learned_clause = {:?}",
                        learned_clause.collect::<Vec<_>>()
                    );
                }
                trail.pop_to_level(level, assignments, inform_decider);
                return result
            }
        }
        PropagationResult::Consistent
    }
}

impl<'a> IntoIterator for &'a VariableAssignment {
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
    pub fn new(assignment: &'a VariableAssignment) -> Self {
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
