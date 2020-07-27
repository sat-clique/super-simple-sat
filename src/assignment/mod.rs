mod literal;

pub use self::literal::{
    Literal,
    VarAssignment,
    Variable,
};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Assignment {
    len_variables: usize,
    len_assigned: usize,
    assignments: HashMap<Variable, VarAssignment>,
}

pub struct OutOfVariables;

impl Assignment {
    pub fn len_variables(&self) -> usize {
        self.len_variables
    }

    pub fn assign(
        &mut self,
        variable: Variable,
        assignment: VarAssignment,
    ) -> Option<VarAssignment> {
        let old_assigned = self.assignments.insert(variable, assignment);
        if old_assigned.is_none() {
            self.len_assigned += 1;
        }
        old_assigned
    }

    pub fn unassign(&mut self, variable: Variable) -> Option<VarAssignment> {
        let old_assignment = self.assignments.remove(&variable);
        if old_assignment.is_some() {
            self.len_assigned -= 1;
        }
        old_assignment
    }

    pub fn resolve(&self, variable: Variable) -> Option<VarAssignment> {
        self.assignments.get(&variable).copied()
    }

    pub fn is_satisfied(&self, literal: Literal) -> Option<bool> {
        let assignment = self.resolve(literal.variable())?.to_bool();
        let result =
            literal.is_positive() && assignment || literal.is_negative() && !assignment;
        Some(result)
    }

    pub fn new_variable(&mut self) -> Variable {
        let new_var = Variable::from_index(self.len_variables)
            .expect("encountered variable index is out of bounds");
        self.len_variables += 1;
        new_var
    }

    pub fn new_chunk_of_variables(
        &mut self,
        amount: usize,
    ) -> Result<usize, OutOfVariables> {
        if amount == 0 {
            return Ok(self.len_variables)
        }
        let last_index = self.len_variables + amount;
        Variable::from_index(last_index).ok_or_else(|| OutOfVariables)?;
        self.len_variables += amount;
        Ok(self.len_variables)
    }

    pub fn next_variable(&self, current_variable: Variable) -> Option<Variable> {
        if self.len_variables == 0 {
            return None
        }
        let next_index = current_variable
            .into_index()
            .wrapping_add(1)
            .wrapping_rem(self.len_variables);
        Some(
            Variable::from_index(next_index)
                .expect("encountered unexpected invalid variable index"),
        )
    }

    pub fn next_unassigned(&self, pivot: Option<Variable>) -> Option<Variable> {
        if self.len_variables == self.len_assigned {
            return None
        }
        let mut pivot = match pivot {
            Some(pivot) => pivot,
            None => {
                return Some(
                    Variable::from_index(0)
                        .expect("encountered unexpected invalid zero-index variable"),
                )
            }
        };
        loop {
            let next_var = self
                .next_variable(pivot)
                .expect("unexpected missing next variable");
            if self.resolve(next_var).is_none() {
                return Some(next_var)
            }
            pivot = next_var;
        }
    }
}

impl<'a> IntoIterator for &'a Assignment {
    type Item = (Variable, Option<VarAssignment>);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

pub struct Iter<'a> {
    assignment: &'a Assignment,
    current: usize,
}

impl<'a> Iter<'a> {
    pub fn new(assignment: &'a Assignment) -> Self {
        Self {
            assignment,
            current: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Variable, Option<VarAssignment>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.assignment.len_variables() {
            return None
        }
        let index = self.current;
        let variable =
            Variable::from_index(index).expect("encountered unexpected invalid variable");
        let assignment = self.assignment.resolve(variable);
        self.current += 1;
        Some((variable, assignment))
    }
}
