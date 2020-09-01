use super::{
    AssignmentError,
    DecisionLevelsAndReasons,
    VariableAssignment,
};
use crate::{
    clause_db::ClauseId,
    decider::InformDecider,
    Literal,
    Variable,
};
use alloc::{
    vec,
    vec::Vec,
};
use bounded::{
    BoundedStack,
    Index,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TrailLimit(u32);

impl Index for TrailLimit {
    fn from_index(index: usize) -> Self {
        assert!(index <= Variable::MAX_LEN);
        Self(index as u32)
    }

    fn into_index(self) -> usize {
        self.0 as usize
    }
}

/// A concrete decision level in the trail.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DecisionLevel(u32);

impl Index for DecisionLevel {
    fn from_index(index: usize) -> Self {
        assert!(index <= Variable::MAX_LEN);
        Self(index as u32)
    }

    fn into_index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub struct TrailLimits {
    limits: Vec<TrailLimit>,
}

impl Default for TrailLimits {
    fn default() -> Self {
        Self {
            limits: vec![TrailLimit(0)],
        }
    }
}

impl TrailLimits {
    /// Pushes a new limit to the trail limits.
    pub fn push(&mut self, new_limit: TrailLimit) -> DecisionLevel {
        let index = self.limits.len();
        self.limits.push(new_limit);
        DecisionLevel::from_index(index)
    }

    /// Returns the last trail limit.
    pub fn last(&self) -> TrailLimit {
        *self
            .limits
            .last()
            .expect("encountered unexpected empty trail limits")
    }

    /// Pops the trail limits to the given decision level.
    pub fn pop_to_level(&mut self, level: DecisionLevel) -> TrailLimit {
        assert!(level.into_index() >= 1);
        assert!(level.into_index() < self.limits.len());
        self.limits.truncate(level.into_index() + 1);
        self.last()
    }

    /// Returns the current decision level.
    pub fn current_decision_level(&self) -> DecisionLevel {
        let index = self.limits.len();
        DecisionLevel::from_index(index)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Trail {
    propagate_head: usize,
    decisions_and_implications: BoundedStack<Literal>,
    limits: TrailLimits,
}

impl Trail {
    /// Returns the current number of variables.
    fn len_variables(&self) -> usize {
        self.decisions_and_implications.capacity()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Errors
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let total_variables = self.len_variables() + new_variables;
        self.decisions_and_implications
            .resize_capacity(total_variables);
    }

    /// Pushes a new decision level and returns it.
    pub fn bump_decision_level(&mut self) -> DecisionLevel {
        let limit = TrailLimit::from_index(self.decisions_and_implications.len());
        self.limits.push(limit)
    }

    /// Returns the current decision level.
    pub fn current_decision_level(&self) -> DecisionLevel {
        self.limits.current_decision_level()
    }

    /// Returns `true` if the propagation queue is empty.
    fn is_propagation_queue_empty(&self) -> bool {
        if self.decisions_and_implications.is_empty() {
            return true
        }
        self.propagate_head == self.decisions_and_implications.len()
    }

    /// Returns the next literal from the propagation queue if any.
    pub fn pop_enqueued(&mut self) -> Option<Literal> {
        if self.is_propagation_queue_empty() {
            return None
        }
        let popped = self.decisions_and_implications[self.propagate_head];
        self.propagate_head += 1;
        Some(popped)
    }

    /// Pushes a new literal to the trail.
    ///
    /// This does not yet propagate the pushed literal.
    ///
    /// # Errors
    ///
    /// - If the pushed literal is in conflict with the current assignment.
    /// - If the literal has already been assigned.
    pub fn push(
        &mut self,
        literal: Literal,
        reason: Option<ClauseId>,
        assignment: &mut VariableAssignment,
        levels_and_reasons: &mut DecisionLevelsAndReasons,
    ) -> Result<(), AssignmentError> {
        match assignment.is_conflicting(literal) {
            Some(true) => return Err(AssignmentError::Conflict),
            Some(false) => return Err(AssignmentError::AlreadyAssigned),
            None => (),
        }
        self.decisions_and_implications
            .push(literal)
            .expect("encountered unexpected invalid variable");
        assignment.assign(literal.variable(), literal.assignment());
        levels_and_reasons.update(literal.variable(), self.current_decision_level(), reason);
        Ok(())
    }

    /// Backjumps the trail to the given decision level.
    pub fn pop_to_level(
        &mut self,
        level: DecisionLevel,
        assignments: &mut VariableAssignment,
        mut inform_decider: InformDecider,
    ) {
        let level = DecisionLevel::from_index(level.into_index() - 1);
        let limit = self.limits.pop_to_level(level);
        self.propagate_head = limit.into_index();
        self.decisions_and_implications
            .pop_to(limit.into_index(), |popped| {
                let variable = popped.variable();
                assignments.unassign(variable);
                inform_decider.restore_variable(variable)
            });
    }
}
