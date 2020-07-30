pub use super::{
    Assignment,
    Error,
    Literal,
    VarAssignment,
    Variable,
};
use crate::utils::{
    bounded_bitmap,
    BoundedBitmap,
};
use core::{
    fmt,
    fmt::Display,
    iter,
    slice,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LastModel {
    last_model: Option<Model>,
}

impl LastModel {
    pub fn update(&mut self, assignment: &Assignment) -> Result<(), Error> {
        match &mut self.last_model {
            Some(last_model) => {
                last_model
                    .from_reuse(&assignment)
                    .expect("encountered unexpected incomplete assignment");
            }
            none => {
                *none = Some(
                    Model::new(&assignment)
                        .expect("encountered unexpected incomplete assignment"),
                );
            }
        }
        Ok(())
    }

    pub fn get(&self) -> Option<&Model> {
        self.last_model.as_ref()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Model {
    assignment: BoundedBitmap<Variable, VarAssignment>,
}

impl Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Model (#vars = {})", self.len())?;
        for (variable, assignment) in self {
            let index = variable.into_index();
            let assignment = assignment.to_bool().to_string();
            writeln!(f, " - var({:3}) = {}", index, assignment)?;
        }
        Ok(())
    }
}

impl Model {
    /// Returns the number of assigned variables in the model.
    fn len(&self) -> usize {
        self.assignment.len()
    }

    pub(crate) fn new(assignment: &Assignment) -> Result<Self, Error> {
        let mut model = Self {
            assignment: Default::default(),
        };
        model.from_reuse(assignment)?;
        Ok(model)
    }

    pub(crate) fn from_reuse(&mut self, assignment: &Assignment) -> Result<(), Error> {
        if !assignment.is_assignment_complete() {
            return Err(Error::IndeterminateAssignment)
        }
        self.assignment.increase_len(assignment.len_variables());
        for (variable, var_assignment) in assignment {
            let var_assignment =
                var_assignment.expect("encountered unexpected indeterminate assignment");
            self.assignment
                .set(variable, var_assignment)
                .map_err(|_| Error::VariableIndexOutOfRange);
        }
        Ok(())
    }

    fn resolve(&self, variable: Variable) -> Result<VarAssignment, Error> {
        self.assignment
            .get(variable)
            .map_err(|_| Error::VariableIndexOutOfRange)
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
    iter: iter::Enumerate<bounded_bitmap::Iter<'a, Variable, VarAssignment>>,
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
                    assignment,
                ))
            }
        }
    }
}
