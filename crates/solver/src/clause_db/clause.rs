use crate::Literal;
use core::{
    iter,
    slice,
};
use hashbrown::HashSet;
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyClause,
    SelfConflictingClause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    literals: Vec<Literal>,
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
            return Err(Error::EmptyClause)
        }
        literals.sort_unstable();
        literals.dedup();
        let mut occurences = HashSet::with_capacity(literals.len());
        for &literal in &literals {
            if occurences.contains(&!literal) {
                return Err(Error::SelfConflictingClause)
            }
            occurences.insert(literal);
        }
        Ok(Self { literals })
    }

    /// Returns the first literal of the clause if the clause is a unit clause.
    ///
    /// Otherwise returns `None`.
    pub fn unit_literal(&self) -> Option<Literal> {
        if self.len() == 1 {
            return Some(self.literals[0])
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Sign,
        Variable,
    };

    #[test]
    fn new_empty_clause_fails() {
        assert_eq!(Clause::new(vec![]), Err(Error::EmptyClause));
    }

    #[test]
    fn new_self_conflicting_clause_fails() {
        let var = Variable::from_index(0).unwrap();
        let a1 = var.into_literal(Sign::True);
        let a2 = var.into_literal(Sign::False);
        assert_eq!(Clause::new(vec![a1, a2]), Err(Error::SelfConflictingClause));
    }

    #[test]
    fn new_unit_clause_works() {
        let var = Variable::from_index(0).unwrap();
        let lit = var.into_literal(Sign::True);
        assert!(Clause::new(vec![lit]).is_ok());
    }

    #[test]
    fn new_complex_clause_works() {
        let a = Variable::from_index(0).unwrap();
        let b = Variable::from_index(1).unwrap();
        let c = Variable::from_index(2).unwrap();
        let pa = a.into_literal(Sign::True);
        let pb = b.into_literal(Sign::True);
        let nc = c.into_literal(Sign::False);
        assert!(Clause::new(vec![pa, pb, nc]).is_ok());
    }

    #[test]
    fn new_clause_with_duplicate_lits_works() {
        let var = Variable::from_index(0).unwrap();
        let lit = var.into_literal(Sign::True);
        let clause = Clause::new(vec![lit, lit]).unwrap();
        assert_eq!(clause.len(), 1);
    }

    #[test]
    fn clause_iter_works() {
        let a = Variable::from_index(0).unwrap();
        let b = Variable::from_index(1).unwrap();
        let c = Variable::from_index(2).unwrap();
        let pa = a.into_literal(Sign::True);
        let pb = b.into_literal(Sign::True);
        let nc = c.into_literal(Sign::False);
        let clause = Clause::new(vec![pa, pb, nc]).unwrap();
        let lits = clause.into_iter().collect::<Vec<_>>();
        assert!(lits.contains(&pa));
        assert!(lits.contains(&pb));
        assert!(lits.contains(&nc));
    }
}
