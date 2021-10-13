use crate::{
    assignment::PartialAssignment,
    literal::RegisterVariables,
    Variable,
};
use bounded::{
    BoundedHeap,
    Index as _,
};
use core::{
    convert::identity,
    ops::Add,
};

/// The priority of a variable used for branching decisions.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u64);

impl Add<u64> for Priority {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

/// Restores the variable for the decision heuristic with its original priority.
///
/// # Note
///
/// Implemented by the decision heuristic in order to be informed during backtracking.
pub trait RestoreVariable {
    /// Restores the variable for the decision heuristic with its original priority.
    ///
    /// # Note
    ///
    /// Does nothing if the variable is already restored.
    ///
    /// # Panics
    ///
    /// Implementers may panic if the variable has not been registered.
    fn restore_variable(&mut self, variable: Variable);
}

impl RestoreVariable for Decider {
    #[inline]
    fn restore_variable(&mut self, variable: Variable) {
        self.priorities
            .push_or_update(variable, identity)
            .unwrap_or_else(|_| panic!("encountered invalid variable {}", variable));
    }
}

/// Heuristic that chooses the next literal to propagate.
#[derive(Debug, Default, Clone)]
pub struct Decider {
    len_variables: usize,
    priorities: BoundedHeap<Variable, Priority>,
    _activity_delta: u64,
}

impl RegisterVariables for Decider {
    fn register_variables(&mut self, additional: usize) {
        let total_variables = self.len_variables() + additional;
        self.priorities.resize_capacity(total_variables);
        for i in self.len_variables()..total_variables {
            let variable = Variable::from_index(i);
            self.priorities
                .push_or_update(variable, identity)
                .expect("unexpected variable index out of bounds");
        }
        self.len_variables += additional;
    }
}

impl Decider {
    /// Returns the number of registered variables.
    fn len_variables(&self) -> usize {
        self.len_variables
    }

    /// Bumps the priority of the given variable by a given amount.
    pub fn bump_priority_by(&mut self, variable: Variable, amount: u64) {
        self.priorities
            .push_or_update(variable, |old_weight| old_weight + amount)
            .expect("encountered unexpected out of bounds variable");
    }

    /// Returns the next variable to propgate if any unassigned variable is left.
    ///
    /// This removes the variable from the priority queue.
    pub fn next_unassigned(
        &mut self,
        assignment: &PartialAssignment,
    ) -> Option<Variable> {
        loop {
            let next = self.priorities.pop().map(|(variable, _priority)| variable);
            match next {
                Some(next) => {
                    if assignment.get(next).is_none() {
                        return Some(next)
                    }
                }
                None => return None,
            }
        }
    }
}
