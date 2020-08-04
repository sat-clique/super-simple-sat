use crate::{
    assignment2::VariableAssignment,
    Variable,
};

/// Heuristic that chooses the next literal to propagate.
#[derive(Debug, Default, Clone)]
pub struct Decider {
    len_variables: usize,
}

impl Decider {
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
        self.len_variables += new_variables;
    }

    /// Returns the next literal to propgate if any unassigned variable is left.
    pub fn next_unassigned(&self, assignment: &VariableAssignment) -> Option<Variable> {
        for variable in 0..self.len_variables() {
            let variable = Variable::from_index(variable)
                .expect("encountered unexpected invalid variable index");
            if assignment.get(variable).is_none() {
                return Some(variable)
            }
        }
        None
    }
}
