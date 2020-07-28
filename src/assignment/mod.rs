mod literal;

pub use self::literal::{
    Literal,
    VarAssignment,
    Variable,
};
use core::{
    iter,
    mem,
    slice,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Model {
    assignment: Vec<VarAssignment>,
}

impl Model {
    pub(crate) fn new(assignment: &Assignment) -> Result<Self, Error> {
        if !assignment.is_assignment_complete() {
            return Err(Error::IndeterminateAssignment)
        }
        let assignment = assignment
            .assignments
            .iter()
            .copied()
            .map(|assign| assign.ok_or_else(|| Error::IndeterminateAssignment))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { assignment })
    }

    pub(crate) fn from_reuse(&mut self, assignment: &Assignment) -> Result<(), Error> {
        if !assignment.is_assignment_complete() {
            return Err(Error::IndeterminateAssignment)
        }
        self.assignment
            .resize_with(assignment.len_variables(), || VarAssignment::False);
        self.assignment.clear();
        self.assignment
            .extend(assignment.assignments.iter().copied().map(|assign| {
                assign.expect("encountered unexpected indeterminate assignment")
            }));
        Ok(())
    }

    fn resolve(&self, variable: Variable) -> Result<VarAssignment, Error> {
        self.assignment
            .get(variable.into_index())
            .copied()
            .ok_or_else(|| Error::VariableIndexOutOfRange)
    }

    pub fn is_satisfied(&self, literal: Literal) -> Result<bool, Error> {
        let assignment = self.resolve(literal.variable())?.to_bool();
        let result =
            literal.is_positive() && assignment || literal.is_negative() && !assignment;
        Ok(result)
    }
}

impl<'a> IntoIterator for &'a Model {
    type Item = (Variable, VarAssignment);
    type IntoIter = ModelIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModelIter::new(self)
    }
}

pub struct ModelIter<'a> {
    iter: iter::Enumerate<slice::Iter<'a, VarAssignment>>,
}

impl<'a> ModelIter<'a> {
    pub fn new(model: &'a Model) -> Self {
        Self {
            iter: model.assignment.iter().enumerate(),
        }
    }
}

impl<'a> Iterator for ModelIter<'a> {
    type Item = (Variable, VarAssignment);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some((index, assignment)) => {
                Some((
                    Variable::from_index(index)
                        .expect("encountered unexpected invalid variable index"),
                    *assignment,
                ))
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Assignment {
    len_assigned: usize,
    assignments: Vec<Option<VarAssignment>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UsedTooManyVariables,
    VariableIndexOutOfRange,
    IndeterminateAssignment,
}

impl Assignment {
    fn len_variables(&self) -> usize {
        self.assignments.len()
    }

    fn is_assignment_complete(&self) -> bool {
        self.len_variables() == self.len_assigned
    }

    fn assign_impl(
        &mut self,
        variable: Variable,
        new_assignment: Option<VarAssignment>,
    ) -> Result<Option<VarAssignment>, Error> {
        let assignment = self
            .assignments
            .get_mut(variable.into_index())
            .ok_or_else(|| Error::VariableIndexOutOfRange)?;
        let old_assignment = mem::replace(assignment, new_assignment);
        if new_assignment.is_some() && old_assignment.is_none() {
            self.len_assigned += 1;
        }
        if new_assignment.is_none() && old_assignment.is_some() {
            self.len_assigned -= 1;
        }
        Ok(old_assignment)
    }

    pub fn assign(
        &mut self,
        variable: Variable,
        new_assignment: VarAssignment,
    ) -> Result<Option<VarAssignment>, Error> {
        self.assign_impl(variable, Some(new_assignment))
    }

    pub fn unassign(
        &mut self,
        variable: Variable,
    ) -> Result<Option<VarAssignment>, Error> {
        self.assign_impl(variable, None)
    }

    pub fn resolve(&self, variable: Variable) -> Result<Option<VarAssignment>, Error> {
        self.assignments
            .get(variable.into_index())
            .copied()
            .ok_or_else(|| Error::VariableIndexOutOfRange)
    }

    pub fn is_satisfied(&self, literal: Literal) -> Result<Option<bool>, Error> {
        let result = self
            .resolve(literal.variable())?
            .map(VarAssignment::to_bool)
            .map(|assignment| {
                literal.is_positive() && assignment
                    || literal.is_negative() && !assignment
            });
        Ok(result)
    }

    /// Registers the given amount of additional variables.
    pub fn register_variables(&mut self, amount: usize) -> Result<(), Error> {
        if amount == 0 {
            return Ok(())
        }
        let new_len = self.len_variables() + amount;
        if !Variable::is_valid_index(new_len - 1) {
            return Err(Error::UsedTooManyVariables)
        }
        self.assignments.resize_with(new_len, Default::default);
        Ok(())
    }

    fn next_variable(&self, current_variable: Variable) -> Option<Variable> {
        if self.len_variables() == 0 {
            return None
        }
        let next_index = current_variable
            .into_index()
            .wrapping_add(1)
            .wrapping_rem(self.len_variables());
        Some(
            Variable::from_index(next_index)
                .expect("encountered unexpected invalid variable index"),
        )
    }

    pub fn next_unassigned(
        &self,
        pivot: Option<Variable>,
    ) -> Result<Option<Variable>, Error> {
        if self.len_variables() == self.len_assigned {
            return Ok(None)
        }
        let mut pivot = match pivot {
            Some(pivot) => pivot,
            None => {
                return Ok(Some(
                    Variable::from_index(0)
                        .expect("encountered unexpected invalid zero-index variable"),
                ))
            }
        };
        loop {
            let next_var = self
                .next_variable(pivot)
                .expect("unexpected missing next variable");
            if self.resolve(next_var)?.is_none() {
                return Ok(Some(next_var))
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
    iter: iter::Enumerate<slice::Iter<'a, Option<VarAssignment>>>,
}

impl<'a> Iter<'a> {
    pub fn new(assignment: &'a Assignment) -> Self {
        Self {
            iter: assignment.assignments.iter().enumerate(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Variable, Option<VarAssignment>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some((index, assignment)) => {
                Some((
                    Variable::from_index(index)
                        .expect("encountered unexpected invalid variable index"),
                    *assignment,
                ))
            }
        }
    }
}
