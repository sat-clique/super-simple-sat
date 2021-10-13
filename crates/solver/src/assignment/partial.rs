use super::AssignmentIter;
use crate::{
    Literal,
    RegisterVariables,
    Sign,
    Variable,
};
use bounded::{
    bounded_map,
    Bool,
    BoundedMap,
};
use core::ops::Not;

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

impl<'a> IntoIterator for &'a PartialAssignment {
    type Item = (Variable, Sign);
    type IntoIter = AssignmentIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AssignmentIter::new(self)
    }
}
