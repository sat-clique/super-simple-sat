use crate::{
    clause_db::ClauseId,
    Literal,
    Variable,
};
use core::slice;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UsedTooManyVariables,
    VariableIndexOutOfRange,
}

#[derive(Debug, Default, Clone)]
pub struct OccurrenceMap {
    occurences: Vec<Occurrences>,
}

/// Occurrences of a single variable.
#[derive(Debug, Default, Clone)]
struct Occurrences {
    /// Occurrences with the literal of positive polarity.
    pos: Vec<ClauseId>,
    /// Occurrences with the literal of negative polarity.
    neg: Vec<ClauseId>,
}

impl Occurrences {
    /// Returns the number of positive and negative occurrences of the variable.
    pub fn len_pos_neg(&self) -> (usize, usize) {
        (self.pos.len(), self.neg.len())
    }

    /// Registers the clause identifier for the given literal.
    pub fn register_for_lit(&mut self, literal: Literal, id: ClauseId) {
        match literal.is_positive() {
            true => self.pos.push(id),
            false => self.neg.push(id),
        }
    }

    /// Returns all clauses containing the given literal.
    pub fn get(&self, literal: Literal) -> ClauseIdIter {
        match literal.is_positive() {
            true => ClauseIdIter::new(&self.pos[..]),
            false => ClauseIdIter::new(&self.neg[..]),
        }
    }
}

impl OccurrenceMap {
    /// Returns the number of currently registered variables.
    fn len_variables(&self) -> usize {
        self.occurences.len()
    }

    /// Registers the given amount of additional variables.
    pub fn register_variables(&mut self, amount: usize) -> Result<(), Error> {
        let new_len = self.len_variables() + amount;
        if !Variable::is_valid_index(new_len - 1) {
            return Err(Error::UsedTooManyVariables)
        }
        self.occurences.resize_with(new_len, Default::default);
        Ok(())
    }

    /// Returns the number of positive and negative literals occurrences of the variable.
    pub fn len_pos_neg(&self, variable: Variable) -> Result<(usize, usize), Error> {
        self.occurences
            .get(variable.into_index())
            .map(|occurrences| occurrences.len_pos_neg())
            .ok_or_else(|| Error::VariableIndexOutOfRange)
    }

    /// Registers the given clause identifier for the literal.
    ///
    /// # Note
    ///
    /// This means that the clause associated with the given identifier contains
    /// the literal with the given polarity.
    pub fn register_for_lit(
        &mut self,
        literal: Literal,
        id: ClauseId,
    ) -> Result<(), Error> {
        self.occurences
            .get_mut(literal.variable().into_index())
            .map(|occurrences| occurrences.register_for_lit(literal, id))
            .ok_or_else(|| Error::VariableIndexOutOfRange)
    }

    /// Returns an iterator over all clauses that contain the given literal.
    pub fn iter_potentially_conflicting_clauses(&self, literal: Literal) -> ClauseIdIter {
        self.occurences
            .get(literal.variable().into_index())
            .map(|occurrences| occurrences.get(!literal))
            .unwrap_or_else(|| ClauseIdIter::new(&[]))
    }
}

#[derive(Debug)]
pub struct ClauseIdIter<'a> {
    iter: slice::Iter<'a, ClauseId>,
}

impl<'a> ClauseIdIter<'a> {
    pub fn new(clause_ids: &'a [ClauseId]) -> Self {
        Self {
            iter: clause_ids.iter(),
        }
    }
}

impl<'a> Iterator for ClauseIdIter<'a> {
    type Item = ClauseId;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }
}
