use crate::ClauseDb;
use core::convert::TryFrom;
use core::num::NonZeroI32;
use core::num::NonZeroU32;
use core::ops::Not;
use std::collections::HashMap;

/// A literal of a variable with its polarity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Literal {
    value: NonZeroI32,
}

impl Literal {
    /// Returns the variable of the literal.
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns `true` if the literal has negative polarity.
    pub fn is_negative(self) -> bool {
        self.value.get().is_negative()
    }

    /// Returns `true` if the literal has positive polarity.
    pub fn is_positive(self) -> bool {
        self.value.get().is_positive()
    }

    /// Returns the literal's variable and polarity.
    pub fn into_var_and_assignment(self) -> (Variable, VarAssignment) {
        (
            self.variable(),
            match self.is_positive() {
                true => VarAssignment::True,
                false => VarAssignment::False,
            },
        )
    }
}

impl Not for Literal {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self {
            value: NonZeroI32::new(-self.value.get())
                .expect("encountered zero i32 from non-zero i32"),
        }
    }
}

impl From<cnf_parser::Literal> for Literal {
    fn from(literal: cnf_parser::Literal) -> Self {
        Self {
            value: literal.into_value(),
        }
    }
}

/// A unique variable.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    value: NonZeroU32,
}

impl From<Literal> for Variable {
    fn from(literal: Literal) -> Self {
        Self {
            value: NonZeroU32::new(literal.value.get().abs() as u32)
                .expect("encountered unexpected zero i32"),
        }
    }
}

impl Variable {
    /// Returns the variable for the given index if valid.
    ///
    /// # Note
    ///
    /// This solver only supports up to 2^31-1 unique variables.
    /// Any index that is out of this range is invalid for this operation.
    pub fn from_index(index: usize) -> Option<Self> {
        let index = i32::try_from(index).ok()?;
        NonZeroU32::new((index as u32).wrapping_add(1)).map(|shifted_index| Self {
            value: shifted_index,
        })
    }

    /// Returns the literal for the variable with the given polarity.
    pub fn into_literal(self, assignment: VarAssignment) -> Literal {
        let value = match assignment {
            VarAssignment::True => self.value.get() as i32,
            VarAssignment::False => -1 * self.value.get() as i32,
        };
        Literal {
            value: NonZeroI32::new(value).expect("encountered unexpected zero i32"),
        }
    }

    /// Returns the index of the variable.
    pub fn into_index(self) -> usize {
        self.value.get() as usize - 1
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Assignment {
    len_variables: usize,
    len_assigned: usize,
    assignments: HashMap<Variable, VarAssignment>,
}

pub struct OutOfVariables;

impl Assignment {
    pub fn from_db(clauses: &ClauseDb) -> Self {
        let mut assignment = Self::default();
        assignment.len_variables = clauses.len();
        assignment
    }

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
        let result = literal.is_positive() && assignment || literal.is_negative() && !assignment;
        Some(result)
    }

    pub fn new_variable(&mut self) -> Variable {
        let new_var = Variable::from_index(self.len_variables)
            .expect("encountered variable index is out of bounds");
        self.len_variables += 1;
        new_var
    }

    pub fn new_chunk_of_variables(&mut self, len: usize) -> Result<usize, OutOfVariables> {
        if len == 0 {
            return Ok(self.len_variables)
        }
        let last_index = self.len_variables + len;
        Variable::from_index(last_index).ok_or_else(|| OutOfVariables)?;
        self.len_variables += len;
        Ok(self.len_variables)
    }

    pub fn next_variable(&self, current_variable: Variable) -> Option<Variable> {
        if self.len_variables == 0 {
            return None;
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
            return None;
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
                return Some(next_var);
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
            return None;
        }
        let index = self.current;
        let variable =
            Variable::from_index(index).expect("encountered unexpected invalid variable");
        let assignment = self.assignment.resolve(variable);
        self.current += 1;
        Some((variable, assignment))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VarAssignment {
    True,
    False,
}

impl VarAssignment {
    pub fn to_bool(self) -> bool {
        match self {
            Self::True => true,
            Self::False => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_assignment_to_bool_works() {
        assert_eq!(VarAssignment::True.to_bool(), true);
        assert_eq!(VarAssignment::False.to_bool(), false);
    }

    #[test]
    fn new_variable_works() {
        let var = Variable::from_index(0).unwrap();
        assert_eq!(var.into_index(), 0);
    }

    #[test]
    fn new_variable_fails() {
        assert!(Variable::from_index(u32::MAX as usize).is_none());
        assert!(Variable::from_index(i32::MAX as usize + 1).is_none());
        assert!(Variable::from_index(i32::MAX as usize).is_some());
    }

    #[test]
    fn lit_from_var_works() {
        let var = Variable::from_index(0).unwrap();
        let p = var.into_literal(VarAssignment::True);
        assert!(p.is_positive());
        assert_eq!(p.variable(), var);
        let n = var.into_literal(VarAssignment::False);
        assert!(n.is_negative());
        assert_eq!(n.variable(), var);
    }

    #[test]
    fn lit_not_works() {
        let var = Variable::from_index(0).unwrap();
        let p = var.into_literal(VarAssignment::True);
        assert!(p.is_positive());
        assert_eq!(p.variable(), var);
        let n = !p;
        assert!(n.is_negative());
        assert_eq!(n.variable(), var);
    }

    #[test]
    fn lit_not_not_works() {
        let var = Variable::from_index(0).unwrap();
        let p1 = var.into_literal(VarAssignment::True);
        let p2 = !!p1;
        assert_eq!(p1, p2);
    }
}
