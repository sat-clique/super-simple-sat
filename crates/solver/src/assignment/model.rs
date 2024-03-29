use super::PartialAssignment;
pub use super::{
    Assignment,
    AssignmentError,
};
use crate::{
    Bool,
    Literal,
    Sign,
    Variable,
};
use bounded::{
    bounded_bitmap,
    BoundedBitmap,
    Index as _,
};
use core::{
    fmt,
    fmt::Display,
    iter,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LastModel {
    last_model: Model,
}

impl LastModel {
    /// Updates the model given the current complete assignment.
    ///
    /// # Errors
    ///
    /// If the given assignment is not complete.
    pub fn update(
        &mut self,
        assignment: &PartialAssignment,
    ) -> Result<(), AssignmentError> {
        self.last_model
            .update(assignment)
            .expect("encountered unexpected incomplete assignment");
        Ok(())
    }

    /// Returns the latest model.
    pub fn get(&self) -> &Model {
        &self.last_model
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Model {
    assignment: BoundedBitmap<Variable, Sign>,
}

impl Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut literals = self.into_iter();
        if f.alternate() {
            writeln!(f)?;
            for literal in literals {
                writeln!(f, "  {: >6},", literal)?;
            }
        } else if let Some(first) = literals.next() {
            write!(f, "{}", first)?;
            for rest in literals {
                write!(f, ", {}", rest)?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Model {
    /// Updates the model from the given assignment.
    ///
    /// # Errors
    ///
    /// If the given assignment is not complete.
    pub(crate) fn update(
        &mut self,
        assignment: &PartialAssignment,
    ) -> Result<(), AssignmentError> {
        if !assignment.is_complete() {
            return Err(AssignmentError::IndeterminateAssignment)
        }
        self.assignment.resize_to_len(assignment.len());
        for assiged in assignment {
            self.assignment
                .set(assiged.variable(), assiged.sign())
                .expect("unexpected invalid variable");
        }
        Ok(())
    }

    /// Resolves the assingment of the given variable.
    fn resolve(&self, variable: Variable) -> Result<Sign, AssignmentError> {
        self.assignment
            .get(variable)
            .map_err(|_| AssignmentError::InvalidVariable)
    }

    /// Returns `true` if the given literal is satisfied under this model.
    pub fn is_satisfied(&self, literal: Literal) -> Result<bool, AssignmentError> {
        let assignment = self.resolve(literal.variable())?.into_bool();
        let result = literal.sign().is_pos() && assignment
            || literal.sign().is_neg() && !assignment;
        Ok(result)
    }
}

impl<'a> IntoIterator for &'a Model {
    type Item = Literal;
    type IntoIter = ModelIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModelIter::new(self)
    }
}

pub struct ModelIter<'a> {
    iter: iter::Enumerate<bounded_bitmap::Iter<'a, Variable, Sign>>,
}

impl<'a> ModelIter<'a> {
    pub fn new(model: &'a Model) -> Self {
        Self {
            iter: model.assignment.iter().enumerate(),
        }
    }
}

impl<'a> Iterator for ModelIter<'a> {
    type Item = Literal;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(index, assignment)| {
            Literal::new(Variable::from_index(index), assignment)
        })
    }
}
