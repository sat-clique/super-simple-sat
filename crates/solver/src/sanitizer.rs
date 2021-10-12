use crate::{
    Literal,
    Variable,
};
use core::slice;

/// A slightly faster hash set due to usage of `ahash` hasher.
type HashSet<T> = std::collections::HashSet<T, ahash::RandomState>;

/// A clause sanitizer.
#[derive(Debug, Default, Clone)]
pub struct ClauseSanitizer {
    literals: Vec<Literal>,
    tautologies: HashSet<Variable>,
}

impl ClauseSanitizer {
    /// Sanitized the clause literals.
    ///
    /// # Note
    ///
    /// This removes duplicate literals as well as literals where both polarities occur.
    /// Furthermore this signals empty clauses as well as tautological clauses.
    pub fn sanitize<I, T>(&mut self, literals: I) -> SanitizedLiterals
    where
        I: IntoIterator<IntoIter = T>,
        T: ExactSizeIterator<Item = Literal>,
    {
        let literals = literals.into_iter();
        if literals.len() == 0 {
            return SanitizedLiterals::EmptyClause
        }
        self.literals.clear();
        self.tautologies.clear();
        self.literals.extend(literals);
        self.literals.sort_unstable();
        let tautologies = &mut self.tautologies;
        self.literals.dedup_by(|l, r| {
            if l.variable() == r.variable() {
                if l != r {
                    tautologies.insert(l.variable());
                }
                return true
            }
            false
        });
        self.literals
            .retain(|lit| !tautologies.contains(&lit.variable()));
        match self.literals.split_first() {
            Some((&unit, &[])) => SanitizedLiterals::UnitClause(unit),
            Some(_) => {
                SanitizedLiterals::Literals(LiteralIter {
                    literals: self.literals.iter(),
                })
            }
            None => SanitizedLiterals::TautologicalClause,
        }
    }
}

/// The result for the sanitation of clause literals.
#[derive(Debug, Clone, PartialEq)]
pub enum SanitizedLiterals<'a> {
    /// The input was empty and represents the unsatisfiable empty clause.
    EmptyClause,
    /// A clause that is always satisfied, e.g. `(a OR (NOT a))`.
    TautologicalClause,
    /// The input contained exactly one literal and is unit.
    UnitClause(Literal),
    /// The sanitized inputs is yielded by the literal iterator.
    Literals(LiteralIter<'a>),
}

impl SanitizedLiterals<'_> {
    /// Returns a literals iterator over the literals if any.
    #[cfg(test)]
    pub fn literals(&self) -> LiteralIter {
        match self {
            Self::EmptyClause | Self::TautologicalClause => LiteralIter::default(),
            Self::UnitClause(unit) => LiteralIter::from(unit),
            Self::Literals(literals) => literals.clone(),
        }
    }
}

/// An iterator over the sanitized literals of a clause.
#[derive(Debug, Clone)]
pub struct LiteralIter<'a> {
    literals: slice::Iter<'a, Literal>,
}

impl<'a> Default for LiteralIter<'a> {
    fn default() -> Self {
        LiteralIter {
            literals: [].iter(),
        }
    }
}

impl<'a> From<&'a Literal> for LiteralIter<'a> {
    fn from(unit: &'a Literal) -> Self {
        LiteralIter {
            literals: slice::from_ref(unit).iter(),
        }
    }
}

impl<'a> Iterator for LiteralIter<'a> {
    type Item = Literal;

    fn next(&mut self) -> Option<Self::Item> {
        self.literals.next().copied()
    }
}

impl<'a> ExactSizeIterator for LiteralIter<'a> {
    fn len(&self) -> usize {
        self.literals.len()
    }
}

impl<'a, I> PartialEq<I> for LiteralIter<'a>
where
    I: IntoIterator<Item = Literal> + Clone,
{
    fn eq(&self, other: &I) -> bool {
        self.clone().eq(other.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience function to easily create a vector of literals.
    fn clause<I>(literals: I) -> Vec<Literal>
    where
        I: IntoIterator<Item = i32>,
    {
        literals.into_iter().map(Literal::from).collect::<Vec<_>>()
    }

    #[test]
    fn sanitation_works() {
        let mut sanitizer = ClauseSanitizer::default();
        assert_eq!(sanitizer.sanitize([]), SanitizedLiterals::EmptyClause);
        assert_eq!(
            sanitizer.sanitize(clause([1])),
            SanitizedLiterals::UnitClause(Literal::from(1))
        );
        assert_eq!(
            sanitizer.sanitize(clause([1, -1])),
            SanitizedLiterals::TautologicalClause,
        );
        assert_eq!(
            sanitizer.sanitize(clause([1, 2, 3, 4, 5])).literals(),
            clause([1, 2, 3, 4, 5])
        );
        assert_eq!(
            sanitizer.sanitize(clause([1, 2, 2, 3, 3])).literals(),
            clause([1, 2, 3])
        );
        assert_eq!(
            sanitizer.sanitize(clause([1, 2, -2, 3, -3])).literals(),
            clause([1])
        );
        assert_eq!(
            sanitizer.sanitize(clause([1, 2, 3, -1, -1, -2, -2, -3, -3])),
            SanitizedLiterals::TautologicalClause,
        );
        assert_eq!(
            sanitizer
                .sanitize(clause([1, 2, 3, -1, -1, -2, -2, -3, -3, 4]))
                .literals(),
            clause([4])
        );
    }
}
