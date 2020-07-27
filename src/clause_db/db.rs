use super::Clause;
use core::iter::FromIterator;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ClauseId(usize);

impl ClauseId {
    /// Creates a new clause identifier from the given index.
    fn from_index(id: usize) -> Self {
        Self(id)
    }

    /// Returns the index of the clause identifier.
    fn into_index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct ClauseDb {
    clauses: Vec<Clause>,
}

impl FromIterator<Clause> for ClauseDb {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Clause>,
    {
        Self {
            clauses: iter.into_iter().collect(),
        }
    }
}

impl<'a> IntoIterator for &'a ClauseDb {
    type Item = (ClauseId, &'a Clause);
    type IntoIter = ClauseDbIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ClauseDbIter::new(self)
    }
}

pub struct ClauseDbIter<'a> {
    current: usize,
    iter: core::slice::Iter<'a, Clause>,
}

impl<'a> ClauseDbIter<'a> {
    fn new(clause_db: &'a ClauseDb) -> Self {
        Self {
            current: 0,
            iter: clause_db.clauses.iter(),
        }
    }
}

impl<'a> Iterator for ClauseDbIter<'a> {
    type Item = (ClauseId, &'a Clause);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|clause| {
            let id = ClauseId::from_index(self.current);
            self.current += 1;
            (id, clause)
        })
    }
}

impl ClauseDb {
    /// Returns the number of clauses stored in the clause database.
    pub fn len(&self) -> usize {
        self.clauses.len()
    }

    /// Returns `true` if the clause database is empty.
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Pushes another clause to the clause database, returns its identifier.
    ///
    /// # Note
    ///
    /// The identifier can be used to resolve the clause again.
    pub fn push(&mut self, clause: Clause) -> ClauseId {
        let id = self.len();
        self.clauses.push(clause);
        ClauseId::from_index(id)
    }

    /// Returns the clause associated with the given clause identifier if any.
    pub fn resolve(&self, id: ClauseId) -> Option<&Clause> {
        self.clauses.get(id.into_index())
    }
}
