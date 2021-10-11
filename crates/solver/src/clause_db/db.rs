use super::{
    ClauseRef,
    ClauseRefMut,
};
use crate::Literal;
use bounded::Index;
use core::{
    mem,
    num::NonZeroU32,
    ops::Range,
    slice,
};

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

#[derive(Debug, Default, Clone)]
pub struct ClauseDb {
    ends: Vec<LiteralsEnd>,
    literals: Vec<Literal>,
}

/// A unit clause that cannot be stored in the clause data base.
///
/// # Note
///
/// Unit clauses are instead turned into problem instance assumptions.
#[derive(Debug)]
pub struct UnitClause {
    /// The unit literal of the unit clause.
    pub literal: Literal,
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

    /// Pushes another clause to the clause database, returns its identifier.
    ///
    /// # Note
    ///
    /// The identifier can be used to resolve the clause again.
    ///
    /// # Errors
    ///
    /// If the given clause is a unit clause. In this case the clause is
    /// returned as unit clause for further processing.
    pub fn push<I, T>(&mut self, literals: I) -> Result<ClauseId, UnitClause>
    where
        I: IntoIterator<IntoIter = T>,
        T: ExactSizeIterator<Item = Literal>,
    {
        let mut literals = literals.into_iter();
        if literals.len() == 1 {
            return Err(UnitClause {
                literal: literals.next().unwrap(),
            })
        }
        let id = self.len();
        self.literals.extend(literals);
        let end = self.literals.len();
        self.ends.push(LiteralsEnd::from_index(end));
        Ok(ClauseId::from_index(id))
    }

    /// Pushes another clause to the clause database, returns its identifier.
    ///
    /// # Note
    ///
    /// The identifier can be used to resolve the clause again.
    ///
    /// # Errors
    ///
    /// If the given clause is a unit clause. In this case the clause is
    /// returned as unit clause for further processing.
    pub fn push_get<I, T>(&mut self, literals: I) -> Result<ClauseRef, UnitClause>
    where
        I: IntoIterator<IntoIter = T>,
        T: ExactSizeIterator<Item = Literal>,
    {
        let mut literals = literals.into_iter();
        if literals.len() == 1 {
            return Err(UnitClause {
                literal: literals.next().unwrap(),
            })
        }
        let id = ClauseId::from_index(self.len());
        let start = self.literals.len();
        self.literals.extend(literals);
        let end = self.literals.len();
        self.ends.push(LiteralsEnd::from_index(end));
        let clause_ref = ClauseRef::new(id, &self.literals[start..end])
            .expect("encountered unexpected invalid shared clause reference");
        Ok(clause_ref)
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
        ClauseRef::new(id, &self.literals[self.clause_id_to_literals_range(id)])
            .expect("encountered invalid clause literals")
            .into()
    }

    /// Returns the clause associated with the given clause identifier if any.
    pub fn resolve_mut(&mut self, id: ClauseId) -> Option<ClauseRefMut> {
        if id.into_index() >= self.len() {
            return None
        }
        let literals_range = self.clause_id_to_literals_range(id);
        ClauseRefMut::new(&mut self.literals[literals_range])
            .expect("encountered invalid clause literals")
            .into()
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
                let clause_ref = ClauseRef::new(id, &self.literals[start..end])
                    .expect("encountered invalid literals");
                Some((id, clause_ref))
            }
            None => None,
        }
    }
}
