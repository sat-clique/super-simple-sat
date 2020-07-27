use super::Error;
use crate::Literal;
use core::{
    iter,
    slice,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClauseRef<'a> {
    literals: &'a [Literal],
}

impl<'a> ClauseRef<'a> {
    pub fn new(literals: &'a [Literal]) -> Result<Self, Error> {
        debug_assert!(!literals.is_empty());
        Ok(Self { literals })
    }
}

impl<'a> IntoIterator for ClauseRef<'a> {
    type Item = Literal;
    type IntoIter = iter::Copied<slice::Iter<'a, Literal>>;

    fn into_iter(self) -> Self::IntoIter {
        self.literals.iter().copied()
    }
}
