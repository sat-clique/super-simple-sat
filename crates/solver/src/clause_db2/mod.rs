mod impls;
mod iter;
mod resolved;
mod words;

#[cfg(test)]
mod tests;

use self::words::{
    ClauseLength,
    ClauseWord,
};
pub use self::{
    impls::ClauseRemoval,
    iter::ClauseDatabaseIter,
    resolved::{
        Literals,
        LiteralsMut,
        ResolvedClause,
        ResolvedClauseMut,
    },
    words::{
        ClauseHeader,
        ClauseHeaderBuilder,
    },
};

/// An unresolved reference to a clause stored in the clause database.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClauseRef(u32);

impl ClauseRef {
    /// Returns the `u32` representation of the unresolved clause reference.
    #[inline]
    fn into_u32(self) -> u32 {
        self.0
    }
}

/// A clause database storing clauses with 2 or more literals.
///
/// # Note
///
/// - The clause database stores all the clauses in a contiguous buffer.
/// - A newly allocated clause is appended to the end of the buffer.
/// - A clause in the clause database is always represented with
///   a single `ClauseHeader` word, followed by a single `ClauseLength(n)`
///   word, followed by `n` literal words.
#[derive(Default, Clone)]
pub struct ClauseDatabase {
    /// The buffer where all clause headers, lengths and literals are stored.
    words: Vec<ClauseWord>,
    /// Total amount of words of clauses that have been marked as removed.
    ///
    /// # Note
    ///
    /// This information is valuable to the garbage collector to decide
    /// whether another garbage collection sweep is actually needed.
    freed_words: usize,
    /// Stores the number of clauses stored in the clause database.
    len_clauses: usize,
}
