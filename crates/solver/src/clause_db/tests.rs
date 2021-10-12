use super::*;
use crate::Literal;
use core::mem;

/// Convenience function to easily create a vector of literals.
fn clause<I>(literals: I) -> Vec<Literal>
where
    I: IntoIterator<Item = i32>,
{
    literals.into_iter().map(Literal::from).collect::<Vec<_>>()
}

#[test]
fn clause_words_sizes() {
    let clause_word_size = mem::size_of::<ClauseWord>();
    assert_eq!(clause_word_size, mem::size_of::<ClauseHeader>());
    assert_eq!(clause_word_size, mem::size_of::<ClauseLength>());
    assert_eq!(clause_word_size, mem::size_of::<Literal>());
    // Additionally clause words need to align the same as literals.
    assert_eq!(mem::align_of::<ClauseWord>(), mem::align_of::<Literal>());
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
    assert_eq!(rc1.header(), &ClauseHeader::default());
    assert_eq!(rc1.literals().as_slice(), &clause([1, 2, 3]));
    let rc2 = db.resolve(c2).unwrap();
    assert_eq!(rc2.header(), &ClauseHeader::default());
    assert_eq!(rc2.literals().as_slice(), &clause([-1, -2, -3]));
    let rc3 = db.resolve(c3).unwrap();
    assert_eq!(rc3.header(), &ClauseHeader::default());
    assert_eq!(rc3.literals().as_slice(), &clause([4, 5, 6, 7]));
    assert_eq!(db.remove_clause(c1), ClauseRemoval::Removed(5));
    assert_eq!(db.remove_clause(c2), ClauseRemoval::Removed(5));
    assert!(db.resolve(c1).is_some());
    assert!(db.resolve(c2).is_some());
    assert!(db.resolve(c3).is_some());
    assert!(!db.is_empty());
    assert_eq!(db.remove_clause(c1), ClauseRemoval::AlreadyRemoved);
    assert_eq!(db.remove_clause(c2), ClauseRemoval::AlreadyRemoved);
    let mut changed_ids = Vec::new();
    assert_eq!(db.gc(|from, into| changed_ids.push((from, into))), 10);
    assert_eq!(changed_ids, vec![(ClauseRef(10), ClauseRef(0))]);
    assert!(db.resolve(ClauseRef(10)).is_none());
    let rc3 = db.resolve(ClauseRef(0)).unwrap();
    assert_eq!(rc3.header(), &ClauseHeader::default());
    assert_eq!(rc3.literals().as_slice(), &clause([4, 5, 6, 7]));
    changed_ids.clear();
    assert_eq!(db.gc(|from, into| changed_ids.push((from, into))), 0);
    assert_eq!(changed_ids, vec![]);
}

#[test]
fn resolve_mut_works() {
    let mut db = ClauseDatabase::default();

    let c1 = db.alloc(clause([1, 2, 3]));
    let c2 = db.alloc(clause([4, 5, 6]));

    // Resolve first clause as exclusive reference and change one literal.
    let mut rc1 = db.resolve_mut(c1).unwrap();
    let rc1_lits = rc1.literals_mut().into_slice();
    assert_eq!(rc1_lits, &mut clause([1, 2, 3]));
    rc1_lits[0] = Literal::from(-1);
    assert_eq!(rc1_lits, &mut clause([-1, 2, 3]));

    // Resolve second clause as exclusive reference and flip all literals.
    let mut rc2 = db.resolve_mut(c2).unwrap();
    let rc2_lits = rc2.literals_mut().into_slice();
    assert_eq!(rc2_lits, &mut clause([4, 5, 6]));
    for lit in &mut *rc2_lits {
        *lit = !*lit;
    }
    assert_eq!(rc2_lits, &mut clause([-4, -5, -6]));
}
