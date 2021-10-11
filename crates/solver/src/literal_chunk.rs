use crate::{
    Error,
    Literal,
    Sign,
    Variable,
};

/// A chunk of literals.
///
/// Created by the
/// [`Solver::new_literal_chunk`](`crate::Solver::new_literal_chunk)
/// method.
#[derive(Debug, Clone)]
pub struct LiteralChunk {
    /// The start index of this chunk for the first literal.
    start_index: usize,
    /// The number of literals in the chunk.
    len: usize,
}

impl LiteralChunk {
    /// Creates a new literal chunk for the given start index and length.
    pub(crate) fn new(start_index: usize, end_index: usize) -> Result<Self, Error> {
        if start_index >= end_index {
            return Err(Error::InvalidLiteralChunkRange)
        }
        if !Variable::is_valid_index(start_index) {
            return Err(Error::InvalidLiteralChunkStart)
        }
        if !Variable::is_valid_index(end_index) {
            return Err(Error::InvalidLiteralChunkEnd)
        }
        Ok(Self {
            start_index,
            len: end_index - start_index,
        })
    }

    /// Returns the number of literals in this chunk.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the n-th literal of the chunk if within bounds.
    pub fn get(&self, n: usize) -> Option<Literal> {
        if n >= self.len() {
            return None
        }
        let var = Variable::from_index(self.start_index + n)
            .expect("encountered unexpected out of bounds variable index");
        Some(Literal::new(var, Sign::POS))
    }
}

impl IntoIterator for LiteralChunk {
    type Item = Literal;
    type IntoIter = LiteralChunkIter;

    fn into_iter(self) -> Self::IntoIter {
        LiteralChunkIter::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct LiteralChunkIter {
    current: usize,
    chunk: LiteralChunk,
}

impl LiteralChunkIter {
    fn new(chunk: LiteralChunk) -> Self {
        Self { current: 0, chunk }
    }
}

impl Iterator for LiteralChunkIter {
    type Item = Literal;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.chunk.len() - self.current;
        (remaining, Some(remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        match self.chunk.get(self.current) {
            None => None,
            Some(literal) => {
                self.current += 1;
                Some(literal)
            }
        }
    }
}
