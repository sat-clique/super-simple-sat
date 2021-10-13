use crate::Literal;
use core::{
    fmt,
    fmt::{
        Debug,
        Formatter,
    },
};
use utils::{
    slice_cast,
    slice_cast_mut,
};

/// Represents the length of a clause stored in the clause database.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
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
#[repr(transparent)]
pub struct ClauseHeader {
    inner: u32,
}

/// Builds up a clause header.
#[derive(Debug, Default)]
pub struct ClauseHeaderBuilder {
    /// The clause header under construction.
    inner: ClauseHeader,
}

impl ClauseHeaderBuilder {
    /// Makes the built clause header refer to a deleted clause.
    #[inline]
    pub fn deleted(mut self, is_deleted: bool) -> Self {
        self.inner.set_deleted(is_deleted);
        Self { inner: self.inner }
    }

    /// Makes the built clause header refer to a learned clause.
    #[inline]
    pub fn learnt(mut self, is_learnt: bool) -> Self {
        self.inner.set_learnt(is_learnt);
        Self { inner: self.inner }
    }

    /// Finalizes building of the clause header.
    #[inline]
    pub fn finish(self) -> ClauseHeader {
        self.inner
    }
}

impl Debug for ClauseHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClauseHeader")
            .field("deleted", &self.is_deleted())
            .finish()
    }
}

impl ClauseHeader {
    /// Returns a clause header builder.
    pub fn build() -> ClauseHeaderBuilder {
        ClauseHeaderBuilder {
            inner: Default::default(),
        }
    }

    /// Returns `true` if the clause has been deleted from the clause database.
    ///
    /// # Note
    ///
    /// If a clause stored in the clause database is deleted it won't be removed
    /// right away. Instead it is marked as deleted and removed in the next garbage
    /// collection sweep.
    #[inline]
    pub fn is_deleted(self) -> bool {
        self.inner & 0b01 != 0
    }

    /// Returns `true` if the clause has been deleted from the clause database.
    ///
    /// # Note
    ///
    /// If a clause stored in the clause database is deleted it won't be removed
    /// right away. Instead it is marked as deleted and removed in the next garbage
    /// collection sweep.
    #[inline]
    pub fn is_learnt(self) -> bool {
        self.inner & 0b10 != 0
    }

    /// Marks the clause as deleted.
    ///
    /// # Note
    ///
    /// - This is a private function only to be used by the clause header builder.
    /// - Marking a clause as deleted won't delete the clause right away.
    ///   Instead with the next sweep of the clause database garbage collection
    ///   all clauses that have been marked as deleted are being removed.
    /// - This API shall never be called directly by a user but indirectly
    ///   through the clause database to keep track of state.
    pub(super) fn set_deleted(&mut self, is_deleted: bool) {
        if is_deleted {
            self.inner |= 0b01;
        } else {
            self.inner &= !0b01;
        }
    }

    /// Marks the clause as learnt clause.
    ///
    /// # Note
    ///
    /// This is a private function only to be used by the clause header builder.
    fn set_learnt(&mut self, is_learnt: bool) {
        if is_learnt {
            self.inner |= 0b10;
        } else {
            self.inner &= !0b10;
        }
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
pub union ClauseWord {
    header: ClauseHeader,
    len: ClauseLength,
    lit: Literal,
}

/// Implementation block that allows for the unsafe casting operations.
#[allow(unsafe_code)]
impl ClauseWord {
    /// Interprets the clause word as the clause header.
    pub fn as_header(&self) -> &ClauseHeader {
        // SAFETY: All clause word variants `ClauseHeader`, `ClauseLength` and `Literal`
        //         are based on the `u32` Rust primitive type.
        //         Casting between them does not invalidate internal state.
        //         The clause database guarantees to perform only valid casts.
        unsafe { &self.header }
    }

    /// Interprets the clause word reference as an exclusive reference to the clause header.
    pub fn as_header_mut(&mut self) -> &mut ClauseHeader {
        // SAFETY: All clause word variants `ClauseHeader`, `ClauseLength` and `Literal`
        //         are based on the `u32` Rust primitive type.
        //         Casting between them does not invalidate internal state.
        //         The clause database guarantees to perform only valid casts.
        unsafe { &mut self.header }
    }

    /// Interprets the clause word as the clause length.
    pub fn as_len(self) -> usize {
        // SAFETY: All clause word variants `ClauseHeader`, `ClauseLength` and `Literal`
        //         are based on the `u32` Rust primitive type.
        //         Casting between them does not invalidate internal state.
        //         The clause database guarantees to perform only valid casts.
        unsafe { self.len }.value() as usize
    }
}

impl ClauseWord {
    /// Interprets the slice of words as slice of literals.
    pub fn as_lits(words: &[Self]) -> &[Literal] {
        slice_cast!(<ClauseWord, Literal>(words))
    }

    /// Interprets the slice of words as slice of literals.
    pub fn as_lits_mut(words: &mut [Self]) -> &mut [Literal] {
        slice_cast_mut!(<ClauseWord, Literal>(words))
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

    #[test]
    fn builder_works() {
        fn assert_for(is_learnt: bool, is_deleted: bool) {
            let header = ClauseHeader::build()
                .learnt(is_learnt)
                .deleted(is_deleted)
                .finish();
            assert_eq!(header.is_learnt(), is_learnt);
            assert_eq!(header.is_deleted(), is_deleted);
        }
        assert_for(false, false);
        assert_for(false, true);
        assert_for(true, false);
        assert_for(true, true);
    }

    #[test]
    fn builder_doube_set_works() {
        fn assert_for(is_learnt: bool, is_deleted: bool) {
            let header = ClauseHeader::build()
                .learnt(is_learnt)
                .learnt(is_learnt)
                .deleted(is_deleted)
                .deleted(is_deleted)
                .finish();
            assert_eq!(header.is_learnt(), is_learnt);
            assert_eq!(header.is_deleted(), is_deleted);
        }
        assert_for(false, false);
        assert_for(false, true);
        assert_for(true, false);
        assert_for(true, true);
    }

    #[test]
    fn builder_negate_works() {
        fn assert_for(is_learnt: bool, is_deleted: bool) {
            let header = ClauseHeader::build()
                .learnt(!is_learnt)
                .learnt(is_learnt)
                .deleted(!is_deleted)
                .deleted(is_deleted)
                .finish();
            assert_eq!(header.is_learnt(), is_learnt);
            assert_eq!(header.is_deleted(), is_deleted);
        }
        assert_for(false, false);
        assert_for(false, true);
        assert_for(true, false);
        assert_for(true, true);
    }
}
