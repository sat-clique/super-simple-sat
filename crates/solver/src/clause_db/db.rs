use super::{
    ClauseRef,
    ClauseRefMut,
};
use crate::Literal;
use alloc::vec::Vec;
use bounded::Index;
use core::{
    mem,
    num::NonZeroU32,
    ops::Range,
    slice,
};
use super::VerifiedClause;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ClauseId(NonZeroU32);

impl Index for ClauseId {
    /// Creates a new clause identifier from the given index.
    #[inline]
    fn from_index(id: usize) -> Self {
        Self(
            NonZeroU32::new((id as u32).wrapping_add(1))
                .expect("encountered unexpected out of bounds clause ID"),
        )
    }

    /// Returns the index of the clause identifier.
    #[inline]
    fn into_index(self) -> usize {
        self.0.get().wrapping_sub(1) as usize
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct LiteralsEnd(usize);

impl LiteralsEnd {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

/// Efficiently stores clauses and their literals.
///
/// Allows to access stored clauses via their associated clause identifiers.
#[derive(Debug, Default, Clone)]
pub struct ClauseDb {
    ends: Vec<LiteralsEnd>,
    literals: Vec<Literal>,
}

impl ClauseDb {
    /// Returns the number of clauses stored in the clause database.
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Returns `true` if the clause database is empty.
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// Pushes an already verified clause to the database and returns a reference to it.
    ///
    /// # Note
    ///
    /// Since the clause has already been verified no further checks are required.
    pub fn push_clause(&mut self, clause: VerifiedClause) -> ClauseRef {
        let id = ClauseId::from_index(self.len());
        let start = self.literals.len();
        self.literals.extend_from_slice(clause.literals);
        let end = self.literals.len();
        self.ends.push(LiteralsEnd::from_index(end));
        let clause_literals = &mut self.literals[start..end];
        ClauseRef::new(id, clause_literals)
    }

    /// Converts the clause identifier into the range of its literals.
    fn clause_id_to_literals_range(&self, id: ClauseId) -> Range<usize> {
        let index = id.into_index();
        let start = self
            .ends
            .get(index.wrapping_sub(1))
            .map(|end| end.0)
            .unwrap_or_else(|| 0);
        let end = self.ends[index].into_index();
        start..end
    }

    /// Returns the clause associated with the given clause identifier if any.
    pub fn resolve(&self, id: ClauseId) -> Option<ClauseRef> {
        if id.into_index() >= self.len() {
            return None
        }
        ClauseRef::new(id, &self.literals[self.clause_id_to_literals_range(id)]).into()
    }

    /// Returns the clause associated with the given clause identifier if any.
    pub fn resolve_mut(&mut self, id: ClauseId) -> Option<ClauseRefMut> {
        if id.into_index() >= self.len() {
            return None
        }
        let literals_range = self.clause_id_to_literals_range(id);
        ClauseRefMut::new(&mut self.literals[literals_range]).into()
    }
}

impl<'a> IntoIterator for &'a ClauseDb {
    type Item = (ClauseId, ClauseRef<'a>);
    type IntoIter = ClauseDbIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ClauseDbIter::new(self)
    }
}

pub struct ClauseDbIter<'a> {
    current: usize,
    last_end: usize,
    ends: slice::Iter<'a, LiteralsEnd>,
    literals: &'a [Literal],
}

impl<'a> ClauseDbIter<'a> {
    fn new(clause_db: &'a ClauseDb) -> Self {
        Self {
            current: 0,
            last_end: 0,
            ends: clause_db.ends.iter(),
            literals: &clause_db.literals,
        }
    }
}

impl<'a> Iterator for ClauseDbIter<'a> {
    type Item = (ClauseId, ClauseRef<'a>);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ends.size_hint()
    }

    fn next(&mut self) -> Option<Self::Item> {
        match self.ends.next() {
            Some(end) => {
                let id = ClauseId::from_index(self.current);
                let start = mem::replace(&mut self.last_end, end.into_index());
                let end = end.into_index();
                self.current += 1;
                let clause_ref = ClauseRef::new(id, &self.literals[start..end]);
                Some((id, clause_ref))
            }
            None => None,
        }
    }
}
