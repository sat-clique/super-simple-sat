use super::{
    ClauseDatabase,
    ClauseHeader,
    ClauseLength,
    ClauseRef,
    ClauseWord,
    ResolvedClause,
    ResolvedClauseMut,
};
use crate::Literal;
use core::{
    fmt,
    fmt::{
        Debug,
        Formatter,
    },
    mem,
};

impl ClauseDatabase {
    /// The maximum possible clause reference value.
    const MAX_CLAUSE_REF: usize = u32::MAX as usize;

    /// The maximum possible clause reference value.
    const MAX_CLAUSE_LEN: usize = u32::MAX as usize;

    /// Allocates a new clause to the clause database with the given literals.
    ///
    /// # Panics
    ///
    /// - If the newly allocated clause has less than 2 literals.
    /// - If the newly allocated clause has more literals than allowed.
    /// - If the resulting clause reference would be out of valid bounds.
    pub fn alloc<I, T>(&mut self, literals: I) -> ClauseRef
    where
        I: IntoIterator<IntoIter = T>,
        T: ExactSizeIterator<Item = Literal>,
    {
        let literals = literals.into_iter();
        let len = literals.len();
        assert!(
            len >= 2,
            "encountered short clause with less than 2 literals",
        );
        assert!(
            len < Self::MAX_CLAUSE_LEN,
            "encountered clause with too many literals"
        );
        let current = self.words.len();
        assert!(
            current <= Self::MAX_CLAUSE_REF,
            "out of memory to allocate more clauses"
        );
        self.words.extend(
            [
                ClauseWord::from(ClauseHeader::default()),
                ClauseWord::from(ClauseLength::new(len as u32)),
            ]
            .into_iter()
            .chain(literals.map(ClauseWord::from)),
        );
        self.len_clauses += 1;
        ClauseRef(current as u32)
    }

    /// Returns a shared reference to the clause words if the clause reference was valid.
    #[allow(unsafe_code)]
    fn clause_words(words: &[ClauseWord], cref: ClauseRef) -> Option<ResolvedClause> {
        let index = cref.into_u32() as usize;
        words
            .get(index + 1)
            .copied()
            .map(|word| {
                // SAFETY: While it is not guaranteed that the clause word at
                //         this point refers to the clause length we do a bounds
                //         check later that protects against invalid accesses.
                unsafe { word.as_len() }
            })
            .and_then(|len| words.get(index..(index + len + 2)))
            .map(ResolvedClause::new)
    }

    /// Returns an exclusive reference to the clause words if the clause reference was valid.
    #[allow(unsafe_code)]
    fn clause_words_mut(
        words: &mut [ClauseWord],
        cref: ClauseRef,
    ) -> Option<ResolvedClauseMut> {
        let index = cref.into_u32() as usize;
        words
            .get(index + 1)
            .copied()
            .map(|word| {
                // SAFETY: While it is not guaranteed that the clause word at
                //         this point refers to the clause length we do a bounds
                //         check later that protects against invalid accesses.
                unsafe { word.as_len() }
            })
            .and_then(|len| words.get_mut(index..(index + len + 2)))
            .map(ResolvedClauseMut::new)
    }

    /// Resolves the unresolved clause to a shared reference if it is valid.
    pub fn resolve(&self, cref: ClauseRef) -> Option<ResolvedClause> {
        Self::clause_words(&self.words, cref)
    }

    /// Resolves the unresolved clause to an exclusive reference if it is valid.
    pub fn resolve_mut(&mut self, cref: ClauseRef) -> Option<ResolvedClauseMut> {
        Self::clause_words_mut(&mut self.words, cref)
    }

    /// Marks a clause stored in the clause database as removed.
    ///
    /// # Note
    ///
    /// This won't remove the clause and free its associated data right away.
    /// Instead it will remove the clause upon the next garbage collection sweep.
    pub fn remove_clause(&mut self, cref: ClauseRef) -> ClauseRemoval {
        Self::clause_words_mut(&mut self.words, cref)
            .map(|mut clause| {
                if clause.header().is_deleted() {
                    return ClauseRemoval::AlreadyRemoved
                }
                clause.header_mut().set_deleted(true);
                // Freed words are the words that store the clause header,
                // the clause length as well as a word per clause literal.
                let freed_words = clause.literals().len() + 2;
                self.freed_words += freed_words;
                self.len_clauses -= 1;
                ClauseRemoval::Removed(freed_words)
            })
            .unwrap_or(ClauseRemoval::NotFound)
    }

    /// Removes all clauses marked as deleted from the clause database.
    ///
    /// Returns the amount of freed clause words where every clause word is 32-bit.
    ///
    /// # Note
    ///
    /// - The `report` closure reports to the outside which clause references
    ///   are required to be changed. The first parameter represents the old clause
    ///   reference and the second parameter informs about its replacement.
    /// - After this operation the amount of free clause words in the clause
    ///   database will be equal to zero.
    #[allow(unsafe_code)]
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
            // SAFETY: The `current` index always points to the start of a clause.
            //         Therefore `words[current]` always refers to the clause header.
            let header = unsafe { words[current].as_header() };
            // SAFETY: The `current` index always points to the start of a clause.
            //         Therefore `words[current+1]` always refers to the clause length.
            //
            // # Note
            //
            // The length denotes the amount of literals.
            // Since a clause is also made up of header and length words we need to add 2.
            let clause_len = 2 + unsafe { words[current + 1].as_len() };
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
        self.len() == 0
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
