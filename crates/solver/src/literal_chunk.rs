use crate::{
    Error,
    Literal,
    Sign,
    Variable,
};
use bounded::Index as _;

/// A chunk of literals.
///
/// Created by the
/// [`Solver::new_literal_chunk`]([`crate::Solver::new_literal_chunk`])
/// method.
#[derive(Debug, Clone, Copy)]
pub struct LiteralChunk {
    /// The value of the first literal of the literal chunk.
    first_value: u32,
    /// The number of literals in the literal chunk.
    len: usize,
}

impl LiteralChunk {
    /// Creates a new literal chunk for the given first value and length.
    pub(crate) fn new(first_index: usize, len: usize) -> Result<Self, Error> {
        if !Variable::is_valid_index(first_index + len) {
            return Err(Error::InvalidLiteralChunk)
        }
        // Can now safely cast to `u32`.
        let first_value = first_index as u32;
        Ok(Self { first_value, len })
    }

    /// Returns the number of literals in this chunk.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the literal chunk is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the index of the first literal of the literal chunk.
    fn first_index(&self) -> usize {
        self.first_value as usize
    }

    /// Returns the n-th literal of the chunk if within bounds.
    #[inline]
    pub fn get(&self, n: usize) -> Option<Literal> {
        if n >= self.len() {
            return None
        }
        let var_index = self.first_index() + n;
        let var = Variable::from_index(var_index);
        Some(Literal::new(var, Sign::POS))
    }
}

impl IntoIterator for LiteralChunk {
    type Item = Literal;
    type IntoIter = LiteralChunkIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        LiteralChunkIter::new(self)
    }
}

/// An iterator over the literals of a literal chunk.
#[derive(Debug, Clone, Copy)]
pub struct LiteralChunkIter {
    /// The last yielded literal of the literal chunk.
    current: usize,
    /// The literal chunk that is being iterated over.
    chunk: LiteralChunk,
}

impl LiteralChunkIter {
    /// Creates a new iterator for the literal chunk.
    fn new(chunk: LiteralChunk) -> Self {
        Self { current: 0, chunk }
    }
}

impl Iterator for LiteralChunkIter {
    type Item = Literal;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.chunk.get(self.current).map(|literal| {
            self.current += 1;
            literal
        })
    }
}

impl ExactSizeIterator for LiteralChunkIter {
    #[inline]
    fn len(&self) -> usize {
        self.chunk.len() - self.current
    }
}
