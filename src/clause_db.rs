use crate::Literal;
use core::iter;
use core::iter::FromIterator;
use core::slice;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    literals: Vec<Literal>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyClause,
    SelfConflictingClause,
}

impl Clause {
    /// Creates a new clause from the given literals.
    ///
    /// # Note
    ///
    /// Deduplicates any duplicated literals and sorts them in the process.
    ///
    /// # Errors
    ///
    /// - If the literals are empty.
    /// - If the literals are self conflicting, e.g. `a AND -a`.
    pub fn new<L>(literals: L) -> Result<Self, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        let mut literals = literals.into_iter().collect::<Vec<_>>();
        if literals.is_empty() {
            return Err(Error::EmptyClause);
        }
        literals.sort();
        literals.dedup();
        let mut occurences = HashSet::new();
        for &literal in &literals {
            if occurences.contains(&!literal) {
                return Err(Error::SelfConflictingClause);
            }
            occurences.insert(literal);
        }
        Ok(Self { literals })
    }

    /// Returns the length of the clause.
    pub fn len(&self) -> usize {
        self.literals.len()
    }
}

impl<'a> IntoIterator for &'a Clause {
    type Item = Literal;
    type IntoIter = iter::Copied<slice::Iter<'a, Literal>>;

    fn into_iter(self) -> Self::IntoIter {
        self.literals.iter().copied()
    }
}

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

#[derive(Debug, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VarAssignment, Variable};

    #[test]
    fn new_empty_clause_fails() {
        assert_eq!(Clause::new(vec![]), Err(Error::EmptyClause));
    }

    #[test]
    fn new_self_conflicting_clause_fails() {
        let var = Variable::from_index(0).unwrap();
        let a1 = var.into_literal(VarAssignment::True);
        let a2 = var.into_literal(VarAssignment::False);
        assert_eq!(Clause::new(vec![a1, a2]), Err(Error::SelfConflictingClause));
    }

    #[test]
    fn new_unit_clause_works() {
        let var = Variable::from_index(0).unwrap();
        let lit = var.into_literal(VarAssignment::True);
        assert!(Clause::new(vec![lit]).is_ok());
    }

    #[test]
    fn new_complex_clause_works() {
        let a = Variable::from_index(0).unwrap();
        let b = Variable::from_index(1).unwrap();
        let c = Variable::from_index(2).unwrap();
        let pa = a.into_literal(VarAssignment::True);
        let pb = b.into_literal(VarAssignment::True);
        let nc = c.into_literal(VarAssignment::False);
        assert!(Clause::new(vec![pa, pb, nc]).is_ok());
    }

    #[test]
    fn new_clause_with_duplicate_lits_works() {
        let var = Variable::from_index(0).unwrap();
        let lit = var.into_literal(VarAssignment::True);
        let clause = Clause::new(vec![lit, lit]).unwrap();
        assert_eq!(clause.len(), 1);
    }

    #[test]
    fn clause_iter_works() {
        let a = Variable::from_index(0).unwrap();
        let b = Variable::from_index(1).unwrap();
        let c = Variable::from_index(2).unwrap();
        let pa = a.into_literal(VarAssignment::True);
        let pb = b.into_literal(VarAssignment::True);
        let nc = c.into_literal(VarAssignment::False);
        let clause = Clause::new(vec![pa, pb, nc]).unwrap();
        let lits = clause.into_iter().collect::<Vec<_>>();
        assert!(lits.contains(&pa));
        assert!(lits.contains(&pb));
        assert!(lits.contains(&nc));
    }
}
