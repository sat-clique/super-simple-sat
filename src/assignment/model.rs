pub use super::{
    Literal,
    VarAssignment,
    Variable,
    Assignment,
    Error,
};
use core::{
    iter,
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
