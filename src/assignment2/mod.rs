mod model;
mod trail;
mod watch_list;

use self::{
    model::{
        LastModel,
        Model,
        ModelIter,
    },
    trail::{
        DecisionLevel,
        Trail,
    },
    watch_list::WatchList,
};
use crate::{
    utils::{
        bounded_map,
        BoundedMap,
    },
    ClauseDb,
    Error,
    Literal,
    VarAssignment,
    Variable,
};
use std::collections::VecDeque;

/// Errors that may be encountered when operating on the assignment.
#[derive(Debug)]
pub enum AssignmentError {
    /// When trying to create a model from an indeterminate assignment.
    UnexpectedIndeterminateAssignment,
    /// Variable invalid for the current assignment.
    InvalidVariable,
    /// Initialize an already initialized assignment.
    AlreadyInitialized,
}

/// Allows to enqueue new literals into the propagation queue.
#[derive(Debug)]
pub struct PropagationEnqueuer<'a> {
    queue: &'a mut PropagationQueue,
}

impl<'a> PropagationEnqueuer<'a> {
    /// Returns a new wrapper around the given propagation queue.
    fn new(queue: &'a mut PropagationQueue) -> Self {
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
        assignment: &AssignmentView,
    ) -> Result<(), EnqueueError> {
        self.queue.push(literal, assignment)
    }
}

/// Errors that may be encountered upon enqueuing a literal to the propagation queue.
#[derive(Debug)]
pub enum EnqueueError {
    /// The literal is alreday satisfied and does not need to be propagated.
    AlreadySatisfied,
    /// The literal is in conflict with the current assignment.
    Conflict,
}

impl EnqueueError {
    /// Returns `true` if the enqueue error was caused by a conflict.
    pub fn is_conflict(self) -> bool {
        matches!(self, Self::Conflict)
    }
}

#[derive(Debug, Default)]
pub struct PropagationQueue {
    queue: VecDeque<Literal>,
}

impl PropagationQueue {
    /// Pushes another literal to the propagation queue.
    ///
    /// # Errors
    ///
    /// - If the literal has already been satisfied.
    /// - If the literal is in conflict with the current assignment. This will
    ///   also clear the propagation queue.
    pub fn push(
        &mut self,
        literal: Literal,
        assignment: &AssignmentView,
    ) -> Result<(), EnqueueError> {
        match assignment.get(literal.variable()) {
            Some(VarAssignment::True) => Err(EnqueueError::AlreadySatisfied),
            Some(VarAssignment::False) => {
                self.queue.clear();
                Err(EnqueueError::Conflict)
            }
            None => {
                self.queue.push_back(literal);
                Ok(())
            }
        }
    }

    /// Pops the next propagation literal from the propagation queue.
    pub fn pop(&mut self) -> Option<Literal> {
        self.queue.pop_front()
    }
}

/// Thin-wrapper arround the variable assignment.
///
/// Can only be used to inspect and mutate variable assignment.
///
/// # Note
///
/// Panics for unexpected inputs instead of returning results.
pub struct AssignmentView<'a> {
    assignment: &'a mut BoundedMap<Variable, VarAssignment>,
}

impl<'a> AssignmentView<'a> {
    /// Creates a new mutable assignment wrapper.
    fn new(assignment: &'a mut BoundedMap<Variable, VarAssignment>) -> Self {
        Self { assignment }
    }

    /// Returns the assignment for the given variable.
    ///
    /// # Panics
    ///
    /// If the variable is invalid and cannot be resolved.
    pub fn get(&self, variable: Variable) -> Option<VarAssignment> {
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
            .map(VarAssignment::to_bool)
            .map(|assignment| {
                literal.is_positive() && assignment
                    || literal.is_negative() && !assignment
            })
    }

    /// Updates the assignment of the variable.
    ///
    /// # Panics
    ///
    /// - If the variable is invalid and cannot be resolved.
    /// - If the variable has already been assigned.
    pub fn assign(&mut self, variable: Variable, assignment: VarAssignment) {
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
#[derive(Debug, Default)]
pub struct Assignment {
    is_initialized: bool,
    num_variables: usize,
    trail: Trail,
    assignments: BoundedMap<Variable, VarAssignment>,
    watchers: WatchList,
    propagation_queue: PropagationQueue,
}

impl Assignment {
    /// Initializes the watchers of the assignment given the clause database.
    ///
    /// # Errors
    ///
    /// If the initialization has already taken place.
    pub fn initialize_watchers(&mut self, clause_db: &ClauseDb) -> Result<(), AssignmentError> {
        if self.is_initialized {
            return Err(AssignmentError::AlreadyInitialized)
        }
        self.is_initialized = true;
        for (clause_id, clause) in clause_db {
            for literal in clause {
                self.watchers.register_for_lit(literal, clause_id);
            }
        }
        Ok(())
    }

    /// Returns the current number of variables.
    fn len_variables(&self) -> usize {
        self.num_variables
    }

    /// Returns the number of currently assigned variables.
    fn assigned_variables(&self) -> usize {
        self.assignments.len()
    }

    /// Returns `true` if the assignment is complete.
    fn is_complete(&self) -> bool {
        self.len_variables() == self.assigned_variables()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Errors
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) -> Result<(), Error> {
        let total_variables = self.len_variables() + new_variables;
        self.trail.register_new_variables(new_variables)?;
        self.assignments.increase_capacity_to(total_variables)?;
        self.watchers.register_new_variables(total_variables)?;
        self.num_variables += new_variables;
        Ok(())
    }

    /// Resets the assignment to the given decision level.
    pub fn reset_to_level(&mut self, level: DecisionLevel) {
        let Self {
            trail, assignments, ..
        } = self;
        trail.pop_to_level(level, |popped_lit| {
            assignments
                .take(popped_lit.variable())
                .expect("encountered unexpected invalid unassigned variable");
        })
    }

    /// Enqueues a propagation literal.
    ///
    /// This does not yet perform the actual unit propagation.
    pub fn enqueue_assumption(
        &mut self,
        assumption: Literal,
    ) -> Result<(), EnqueueError> {
        self.propagation_queue
            .push(assumption, &AssignmentView::new(&mut self.assignments))
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
    /// Propagates the enqueued assumptions.
    pub fn propagate(&mut self, clause_db: &mut ClauseDb) -> PropagationResult {
        let Self {
            propagation_queue,
            watchers,
            assignments,
            ..
        } = self;
        while let Some(propagation_literal) = propagation_queue.pop() {
            let result = watchers.propagate(
                propagation_literal,
                clause_db,
                AssignmentView::new(assignments),
                PropagationEnqueuer::new(propagation_queue),
            );
            if result.is_conflict() {
                return result
            }
        }
        PropagationResult::Consistent
    }
}

impl<'a> IntoIterator for &'a Assignment {
    type Item = (Variable, VarAssignment);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

pub struct Iter<'a> {
    iter: bounded_map::Iter<'a, Variable, VarAssignment>,
}

impl<'a> Iter<'a> {
    pub fn new(assignment: &'a Assignment) -> Self {
        Self {
            iter: assignment.assignments.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Variable, VarAssignment);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(variable, assignment)| (variable, *assignment))
    }
}
