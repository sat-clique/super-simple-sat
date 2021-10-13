use super::{
    ClauseDatabase,
    ClauseWord,
    ResolvedClause,
};

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

    #[allow(unsafe_code)]
    fn next(&mut self) -> Option<Self::Item> {
        let words = &mut self.remaining_words;
        if words.is_empty() {
            return None
        }
        // SAFETY: It is guaranteed that the clause word at this point is the clause length.
        let len = unsafe { words[1].as_len_words() };
        let (clause_words, remaining_words) = words.split_at(len);
        *words = remaining_words;
        Some(ResolvedClause::new(clause_words))
    }
}
