use crate::{
    assignment::PartialAssignment,
    Variable,
};
use bounded::{
    BoundedHeap,
    Index as _,
};
use core::ops::Add;

/// The priority of a variable used for branching decisions.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u64);

impl Add<u64> for Priority {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

/// Wrapper around the decider in order to inform it about propagation results.
///
/// This provides an encapsulated interface to the decider that provide access
/// only to the parts that informs it about the variable priorities and which
/// variables are still in need for propagation.
///
/// # Note
///
/// Currently mainly needed to inform the branching heuristic upon backtracking.
#[derive(Debug)]
pub struct InformDecider<'a> {
    /// The wrapped decider.
    decider: &'a mut Decider,
}

impl<'a> InformDecider<'a> {
    /// Wraps the given decider.
    fn new(decider: &'a mut Decider) -> Self {
        Self { decider }
    }

    /// Restores the given variable and adds it back to the priority queue with
    /// its original weight.
    ///
    /// Does nothing if the variable is already in the queue.
    ///
    /// # Panics
    ///
    /// If the given variable index is out of bounds.
    pub fn restore_variable(&mut self, variable: Variable) {
        self.decider.restore_variable(variable)
    }
}

/// Heuristic that chooses the next literal to propagate.
#[derive(Debug, Default, Clone)]
pub struct Decider {
    len_variables: usize,
    priorities: BoundedHeap<Variable, Priority>,
    _activity_delta: u64,
}

impl Decider {
    /// Creates a wrapper around the decider to allow to inform the decider
    /// about unit propagation results.
    pub fn informer(&mut self) -> InformDecider {
        InformDecider::new(self)
    }

    /// Returns the number of registered variables.
    fn len_variables(&self) -> usize {
        self.len_variables
    }

    /// Registers the given amount of new variables.
    ///
    /// # Panics
    ///
    /// If too many variables have been registered in total.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let total_variables = self.len_variables() + new_variables;
        self.priorities.resize_capacity(total_variables);
        for i in self.len_variables()..total_variables {
            let variable = Variable::from_index(i);
            self.priorities
                .push_or_update(variable, core::convert::identity)
                .expect("unexpected variable index out of bounds");
        }
        self.len_variables += new_variables;
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

    /// Restores the given variable and adds it back to the priority queue with
    /// its original weight.
    ///
    /// Does nothing if the variable is already in the queue.
    ///
    /// # Panics
    ///
    /// If the given variable index is out of bounds.
    pub fn restore_variable(&mut self, variable: Variable) {
        self.priorities
            .push_or_update(variable, core::convert::identity)
            .expect("encountered unexpected out of bounds variable");
    }
}
