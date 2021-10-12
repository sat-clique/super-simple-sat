use super::ClauseHeader;
use crate::{
    assignment::VariableAssignment,
    Literal,
};
use core::{
    fmt,
    fmt::{
        Display,
        Formatter,
    },
    ops::{
        Deref,
        DerefMut,
    },
    slice,
};

/// A resolved shared reference to a clause stored in the clause database.
#[derive(Debug, Copy, Clone)]
pub struct ResolvedClause<'a> {
    header: &'a ClauseHeader,
    literals: &'a [Literal],
}

impl<'a> ResolvedClause<'a> {
    /// Creates a new reference to a clause stored in the clause database.
    #[inline]
    pub(super) fn new(header: &'a ClauseHeader, literals: &'a [Literal]) -> Self {
        Self { header, literals }
    }

    /// Returns the header of the referenced clause.
    #[inline]
    pub fn header(&self) -> &'a ClauseHeader {
        self.header
    }

    /// Returns the literals of the referenced clause.
    #[inline]
    pub fn literals(&self) -> Literals<'a> {
        Literals::new(self.literals)
    }
}

/// A shared reference to the literals of a resolved clause.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Literals<'a> {
    literals: &'a [Literal],
}

impl<'a> Deref for Literals<'a> {
    type Target = [Literal];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a> IntoIterator for Literals<'a> {
    type Item = &'a Literal;
    type IntoIter = slice::Iter<'a, Literal>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a> Literals<'a> {
    /// Creates a new literals wrapper around the literal slice.
    fn new(literals: &'a [Literal]) -> Self {
        Self { literals }
    }

    /// Returns a shared reference to the first literal of the resolved clause.
    ///
    /// # Note
    ///
    /// This always yields a literal since all resolved clauses are guaranteed to
    /// have at least two literals.
    #[inline]
    pub fn first(self) -> &'a Literal {
        &self.literals[0]
    }

    /// Returns a shared reference to the second literal of the resolved clause.
    ///
    /// # Note
    ///
    /// This always yields a literal since all resolved clauses are guaranteed to
    /// have at least two literals.
    #[inline]
    pub fn second(self) -> &'a Literal {
        &self.literals[1]
    }

    /// Returns a shared reference to the literal slice.
    #[inline]
    pub fn as_slice(self) -> &'a [Literal] {
        self.literals
    }
}

impl<'a> Display for Literals<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        if let Some((first, rest)) = self.split_first() {
            write!(f, "{}", first)?;
            for lit in rest {
                write!(f, ", {}", lit)?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// A resolved exclusive reference to a clause stored in the clause database.
#[derive(Debug)]
pub struct ResolvedClauseMut<'a> {
    header: &'a mut ClauseHeader,
    literals: &'a mut [Literal],
}

impl<'a> ResolvedClauseMut<'a> {
    /// Creates a new reference to a clause stored in the clause database.
    #[inline]
    pub(super) fn new(header: &'a mut ClauseHeader, literals: &'a mut [Literal]) -> Self {
        Self { header, literals }
    }

    /// Returns the header of the referenced clause.
    #[inline]
    pub fn header(self) -> &'a mut ClauseHeader {
        self.header
    }

    /// Returns the literals of the referenced clause.
    #[inline]
    pub fn literals(self) -> LiteralsMut<'a> {
        LiteralsMut::new(self.literals)
    }
}

/// A shared reference to the literals of a resolved clause.
#[derive(Debug, PartialEq)]
pub struct LiteralsMut<'a> {
    literals: &'a mut [Literal],
}

impl<'a> Deref for LiteralsMut<'a> {
    type Target = &'a mut [Literal];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.literals
    }
}

impl<'a> DerefMut for LiteralsMut<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.literals
    }
}

impl<'a> IntoIterator for LiteralsMut<'a> {
    type Item = &'a mut Literal;
    type IntoIter = slice::IterMut<'a, Literal>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into_slice().iter_mut()
    }
}

impl<'a> LiteralsMut<'a> {
    /// Creates a new literals wrapper around the literal slice.
    fn new(literals: &'a mut [Literal]) -> Self {
        Self { literals }
    }

    /// Returns an exclusive reference to the first literal of the resolved clause.
    ///
    /// # Note
    ///
    /// This always yields a literal since all resolved clauses are guaranteed to
    /// have at least two literals.
    #[inline]
    pub fn into_first(self) -> &'a mut Literal {
        &mut self.literals[0]
    }

    /// Returns an exclusive reference to the second literal of the resolved clause.
    ///
    /// # Note
    ///
    /// This always yields a literal since all resolved clauses are guaranteed to
    /// have at least two literals.
    #[inline]
    pub fn into_second(self) -> &'a mut Literal {
        &mut self.literals[1]
    }

    /// Returns an exlusive reference to the literal slice.
    #[inline]
    pub fn into_slice(self) -> &'a mut [Literal] {
        self.literals
    }
}

impl<'a> Display for LiteralsMut<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        if let Some((first, rest)) = self.split_first() {
            write!(f, "{}", first)?;
            for lit in rest {
                write!(f, ", {}", lit)?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// Result returned from clause local propagation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PropagationResult {
    /// The clause chose a new watched literal.
    NewWatchedLiteral {
        new_watched: Literal,
        new_blocker: Literal,
    },
    /// The clause is now unit under the current assignment.
    UnitUnderAssignment(Literal),
}

impl<'a> LiteralsMut<'a> {
    /// Propagates the propagated literal to the clause literals.
    ///
    /// # Note
    ///
    /// This may change the positions of the clause literals
    /// forcing the watched literals to be on the first two positions.
    ///
    /// Decides whether the clause is unit after the propagation.
    pub fn propagate(
        &mut self,
        propagated_lit: Literal,
        assignment: &VariableAssignment,
    ) -> PropagationResult {
        // Make sure the false literal is in the second [1] position.
        if self.literals[0] == !propagated_lit {
            self.literals.swap(0, 1);
        }
        // Look for new literal to watch:
        for i in 2..self.literals.len() {
            if assignment.is_satisfied(self.literals[i]).unwrap_or(true) {
                self.literals.swap(1, i);
                return PropagationResult::NewWatchedLiteral {
                    new_watched: !self.literals[1],
                    new_blocker: self.literals[0],
                }
            }
        }
        // Clause is unit under current assignment:
        PropagationResult::UnitUnderAssignment(self.literals[0])
    }
}
