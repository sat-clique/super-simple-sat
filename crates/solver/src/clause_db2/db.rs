use crate::Literal;
use core::{
    fmt,
    fmt::{
        Debug,
        Formatter,
    },
    mem,
};

/// A clause database storing clauses with 2 or more literals.
///
/// # Note
///
/// - The clause database stores all the clauses in a contiguous buffer.
/// - A newly allocated clause is appended to the end of the buffer.
/// - A clause in the clause database is always represented with
///   a single `ClauseHeader` word, followed by a single `ClauseLength(n)`
///   word, followed by `n` literal words.
#[derive(Default)]
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

/// An unresolved reference to a clause stored in the clause database.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClauseRef(u32);

impl ClauseRef {
    /// Returns the `u32` representation of the unresolved clause reference.
    #[inline]
    pub fn into_u32(self) -> u32 {
        self.0
    }
}

impl ClauseDatabase {
    /// Allocates a new clause to the clause database with the given literals.
    pub fn alloc<I, T>(&mut self, literals: I) -> ClauseRef
    where
        I: IntoIterator<IntoIter = T>,
        T: ExactSizeIterator<Item = Literal>,
    {
        let literals = literals.into_iter();
        let len = literals.len();
        let current = self.words.len();
        assert!(
            len >= 2,
            "can only allocate clauses with 2 or more literals"
        );
        assert!(
            (current + len) <= u32::MAX as usize,
            "out of memory to allocate more clauses"
        );
        self.words.push(ClauseWord::from(ClauseHeader::default()));
        self.words
            .push(ClauseWord::from(ClauseLength::new(len as u32)));
        for lit in literals {
            self.words.push(ClauseWord::from(lit));
        }
        self.len_clauses += 1;
        ClauseRef(current as u32)
    }

    /// Offsets the internal clause words by the unresolved clause reference.
    ///
    /// # Note
    ///
    /// This has the effect that the returned clause words slice starts
    /// with the referenced clause.
    fn offset_words_by_id(&self, id: ClauseRef) -> Option<&[ClauseWord]> {
        self.words.get(id.into_u32() as usize..)
    }

    /// Offsets the internal clause words by the unresolved clause reference.
    ///
    /// # Note
    ///
    /// This has the effect that the returned clause words slice starts
    /// with the referenced clause.
    fn offset_words_by_id_mut(&mut self, id: ClauseRef) -> Option<&mut [ClauseWord]> {
        self.words.get_mut(id.into_u32() as usize..)
    }

    /// Resolves the unresolved clause reference if it is valid.
    pub fn resolve(&self, id: ClauseRef) -> Option<ResolvedClause> {
        fn resolve_first(remaining_words: &[ClauseWord]) -> Option<ResolvedClause> {
            let header = remaining_words[0].as_header();
            if header.is_deleted() {
                return None
            }
            let len = remaining_words[1].as_len().value() as usize;
            assert!(
                remaining_words.len() >= len + 2,
                "not enough clause words in clause database",
            );
            let literals = ClauseWord::as_lits(&remaining_words[2..len + 2]);
            Some(ResolvedClause::new(header, literals))
        }
        resolve_first(self.offset_words_by_id(id)?)
    }

    /// Marks a clause stored in the clause database as removed.
    ///
    /// # Note
    ///
    /// This won't remove the clause and free its associated data right away.
    /// Instead it will remove the clause upon the next garbage collection sweep.
    pub fn remove_clause(&mut self, id: ClauseRef) -> ClauseRemoval {
        match self.offset_words_by_id_mut(id) {
            Some(words) => {
                let already_removed =
                    mem::replace(&mut words[0].as_header_mut().deleted, true);
                if already_removed {
                    return ClauseRemoval::AlreadyRemoved
                }
                let len = words[1].as_len().value() as usize;
                // Freed words are the words that store the clause header,
                // the clause length as well as a word per clause literal.
                let freed_words = len + 2;
                self.freed_words += freed_words;
                self.len_clauses -= 1;
                ClauseRemoval::Removed(freed_words)
            }
            None => ClauseRemoval::NotFound,
        }
    }

    /// Removes all clauses marked as deleted from the clause database.
    ///
    /// # Note
    ///
    /// - The `report` closure reports to the outside which clause references
    ///   are required to be changed. The first parameter represents the old clause
    ///   reference and the second parameter informs about its replacement.
    /// - After this operation the amount of free clause words in the clause
    ///   database will be equal to zero.
    pub fn gc<F>(&mut self, mut report: F) -> usize
    where
        F: FnMut(ClauseRef, ClauseRef),
    {
        let mut current = 0;
        let mut alive = 0;
        let words = &mut self.words;
        let words_len = words.len();
        loop {
            if current == words.len() {
                break
            }
            let header = words[current].as_header();
            let len = words[current + 1].as_len().value() as usize;
            let clause_len = len + 2;
            if !header.is_deleted() {
                if alive != current {
                    for n in 0..clause_len {
                        // We cannot use `copy_from_slice` since slices might overlap.
                        words[alive + n] = words[current + n];
                    }
                    let from_id = ClauseRef(current as u32);
                    let into_id = ClauseRef(alive as u32);
                    report(from_id, into_id);
                }
                alive += clause_len;
            }
            current += clause_len;
        }
        words.truncate(words_len - self.freed_words);
        mem::replace(&mut self.freed_words, 0)
    }

    /// Returns the amount of living clauses in the clause database.
    ///
    /// # Note
    ///
    /// Clauses marked as deleted that have not yet been sweeped by the
    /// garbage collector are not included in the returned length.
    #[inline]
    pub fn len(&self) -> usize {
        self.len_clauses
    }

    /// Returns `true` if the clause database is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len_clauses == 0
    }
}

/// Tells a user if a clause was successfully removed and how many bytes it freed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClauseRemoval {
    /// The clause has been removed successfully freeing the amount of words.
    Removed(usize),
    /// The clause has already been marked as removed.
    AlreadyRemoved,
    /// The referenced clause does not exist.
    NotFound,
}

impl Debug for ClauseDatabase {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        pub struct DebugClauses<'a> {
            db: &'a ClauseDatabase,
        }

        impl Debug for DebugClauses<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.db).finish()
            }
        }

        let mut db = f.debug_struct("ClauseDatabase");
        db.field("len", &self.words.len());
        db.field("clauses", &DebugClauses { db: self });
        db.finish()
    }
}

/// A resolved reference to a clause stored in the clause database.
#[derive(Debug)]
pub struct ResolvedClause<'a> {
    header: ClauseHeader,
    literals: &'a [Literal],
}

impl<'a> ResolvedClause<'a> {
    /// Creates a new reference to a clause stored in the clause database.
    #[inline]
    pub fn new(header: ClauseHeader, literals: &'a [Literal]) -> Self {
        Self { header, literals }
    }

    /// Returns the header of the referenced clause.
    #[inline]
    pub fn header(&self) -> ClauseHeader {
        self.header
    }

    /// Returns the literals of the referenced clause.
    #[inline]
    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }
}

impl<'a> IntoIterator for &'a ClauseDatabase {
    type IntoIter = ClauseDatabaseIter<'a>;
    type Item = ResolvedClause<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ClauseDatabaseIter {
            remaining_words: self.words.as_slice(),
        }
    }
}

/// An iterator over the clauses stored in the clause database.
pub struct ClauseDatabaseIter<'a> {
    remaining_words: &'a [ClauseWord],
}

impl<'a> Iterator for ClauseDatabaseIter<'a> {
    type Item = ResolvedClause<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let words = &mut self.remaining_words;
        'outer: loop {
            if words.is_empty() {
                return None
            }
            let header = words[0].as_header();
            let len = words[1].as_len().value() as usize;
            let (clause_words, remaining_words) = words.split_at(len + 2);
            *words = remaining_words;
            if header.is_deleted() {
                continue 'outer
            }
            let literals = ClauseWord::as_lits(&clause_words[2..]);
            return Some(ResolvedClause::new(header, literals))
        }
    }
}

/// Represents the length of a clause stored in the clause database.
#[derive(Debug, Copy, Clone)]
pub struct ClauseLength(u32);

impl ClauseLength {
    /// Returns the clause length.
    #[inline]
    pub fn value(self) -> u32 {
        self.0
    }

    /// Creates a new clause legth.
    ///
    /// # Panics
    ///
    /// If `len` is less than 2 since only clauses with 2 or more
    /// literals are allowed.
    #[inline]
    pub fn new(len: u32) -> Self {
        debug_assert!(len >= 2, "clauses must have at least 2 literals");
        Self(len)
    }
}

/// The header of a clause that stores associated clause information.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct ClauseHeader {
    /// Is `true` if the clause has been deleted from the clause database.
    deleted: bool,
    _fill: [u8; 3],
}

impl Debug for ClauseHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClauseHeader")
            .field("deleted", &self.deleted)
            .finish()
    }
}

impl ClauseHeader {
    /// Returns `true` if the clause has been deleted from the clause database.
    ///
    /// # Note
    ///
    /// If a clause stored in the clause database is deleted it won't be removed
    /// right away. Instead it is marked as deleted and removed in the next garbage
    /// collection sweep.
    #[inline]
    pub fn is_deleted(self) -> bool {
        self.deleted
    }
}

/// A 32-bit word of the clause database.
///
/// # Note
///
/// A clause in the clause database is always represented with
/// a single `ClauseHeader` word, followed by a single `ClauseLength(n)`
/// word, followed by `n` literal words.
#[derive(Copy, Clone)]
union ClauseWord {
    header: ClauseHeader,
    len: ClauseLength,
    lit: Literal,
}

impl ClauseWord {
    /// Interprets the clause word as the clause header.
    fn as_header(self) -> ClauseHeader {
        unsafe { self.header }
    }

    /// Interprets the clause word reference as an exclusive reference to the clause header.
    fn as_header_mut(&mut self) -> &mut ClauseHeader {
        unsafe { &mut self.header }
    }

    /// Interprets the clause word as the clause length.
    fn as_len(self) -> ClauseLength {
        unsafe { self.len }
    }

    /// Interprets the slice of words as slice of literals.
    fn as_lits<'a>(words: &'a [Self]) -> &'a [Literal] {
        // The below lines will fail to compile if `ClauseWord` and `Literal`
        // do not have the same `size_of` and `align_of`.
        const _: [(); mem::size_of::<ClauseWord>()] = [(); mem::size_of::<Literal>()];
        const _: [(); mem::align_of::<ClauseWord>()] = [(); mem::align_of::<Literal>()];
        unsafe {
            core::slice::from_raw_parts(words.as_ptr() as *const Literal, words.len())
        }
    }
}

impl From<ClauseHeader> for ClauseWord {
    fn from(header: ClauseHeader) -> Self {
        Self { header }
    }
}

impl From<ClauseLength> for ClauseWord {
    fn from(len: ClauseLength) -> Self {
        Self { len }
    }
}

impl From<Literal> for ClauseWord {
    fn from(lit: Literal) -> Self {
        Self { lit }
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
    fn db_works() {
        let mut db = ClauseDatabase::default();
        assert!(db.is_empty());
        let c1 = db.alloc(clause([1, 2, 3]));
        let c2 = db.alloc(clause([-1, -2, -3]));
        let c3 = db.alloc(clause([4, 5, 6, 7]));
        assert_eq!(db.len(), 3);
        let rc1 = db.resolve(c1).unwrap();
        assert_eq!(rc1.header(), ClauseHeader::default());
        assert_eq!(rc1.literals(), &clause([1, 2, 3]));
        let rc2 = db.resolve(c2).unwrap();
        assert_eq!(rc2.header(), ClauseHeader::default());
        assert_eq!(rc2.literals(), &clause([-1, -2, -3]));
        let rc3 = db.resolve(c3).unwrap();
        assert_eq!(rc3.header(), ClauseHeader::default());
        assert_eq!(rc3.literals(), &clause([4, 5, 6, 7]));
        assert_eq!(db.remove_clause(c1), ClauseRemoval::Removed(5));
        assert_eq!(db.remove_clause(c2), ClauseRemoval::Removed(5));
        assert!(db.resolve(c1).is_none());
        assert!(db.resolve(c2).is_none());
        assert!(db.resolve(c3).is_some());
        assert!(!db.is_empty());
        assert_eq!(db.remove_clause(c1), ClauseRemoval::AlreadyRemoved);
        assert_eq!(db.remove_clause(c2), ClauseRemoval::AlreadyRemoved);
        let mut changed_ids = Vec::new();
        assert_eq!(db.gc(|from, into| changed_ids.push((from, into))), 10);
        assert_eq!(changed_ids, vec![(ClauseRef(10), ClauseRef(0))]);
        assert!(db.resolve(ClauseRef(10)).is_none());
        let rc3 = db.resolve(ClauseRef(0)).unwrap();
        assert_eq!(rc3.header(), ClauseHeader::default());
        assert_eq!(rc3.literals(), &clause([4, 5, 6, 7]));
        changed_ids.clear();
        assert_eq!(db.gc(|from, into| changed_ids.push((from, into))), 0);
        assert_eq!(changed_ids, vec![]);
    }
}
