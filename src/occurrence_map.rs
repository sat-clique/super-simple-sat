use crate::{
    clause_db::ClauseId,
    Literal,
};
use std::collections::{
    HashMap,
    HashSet,
};

#[derive(Debug, Default, Clone)]
pub struct OccurrenceMap {
    empty_dummy: HashSet<ClauseId>,
    occurrences: HashMap<Literal, HashSet<ClauseId>>,
}

impl OccurrenceMap {
    /// Registers the given clause identifier for the literal.
    ///
    /// # Note
    ///
    /// This means that the clause associated with the given identifier contains
    /// the literal with the given polarity.
    pub fn register_for_lit(&mut self, literal: Literal, id: ClauseId) {
        self.occurrences
            .entry(literal)
            .and_modify(|clauses| {
                clauses.insert(id);
            })
            .or_insert_with(|| {
                let mut clauses = HashSet::default();
                clauses.insert(id);
                clauses
            });
    }

    /// Returns an iterator over all clauses that contain the given literal.
    pub fn iter_potentially_conflicting_clauses(&self, literal: Literal) -> ClauseIdIter {
        self.occurrences
            .get(&!literal)
            .map(|clauses| ClauseIdIter::new(clauses))
            .unwrap_or_else(move || ClauseIdIter::new(&self.empty_dummy))
    }
}

#[derive(Debug)]
pub struct ClauseIdIter<'a> {
    iter: std::collections::hash_set::Iter<'a, ClauseId>,
}

impl<'a> ClauseIdIter<'a> {
    pub fn new(literals: &'a HashSet<ClauseId>) -> Self {
        Self {
            iter: literals.iter(),
        }
    }
}

impl<'a> Iterator for ClauseIdIter<'a> {
    type Item = ClauseId;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }
}
