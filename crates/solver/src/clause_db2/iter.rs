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

    fn next(&mut self) -> Option<Self::Item> {
        let words = &mut self.remaining_words;
        'outer: loop {
            if words.is_empty() {
                return None
            }
            let header = words[0].as_header();
            let len = words[1].as_len();
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
