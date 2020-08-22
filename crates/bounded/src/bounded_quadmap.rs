use super::{
    BoundedArray,
    OutOfBoundsAccess,
};
use crate::Index;
use core::marker::PhantomData;

/// A quad that represents one of 4 different states.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum quad {
    /// Both bits are `0`.
    B00 = 0b00,
    /// Least-significant bit is `1`, other is `0`.
    B01 = 0b01,
    /// Most-significant bit is `1`, other is `0`.
    B10 = 0b10,
    /// Both bits are `1`.
    B11 = 0b11,
}

impl Default for quad {
    fn default() -> Self {
        Self::B00
    }
}

/// Types that can convert to and from a [`quad`].
pub trait Quad {
    /// Converts a quad into `self`.
    fn from_quad(value: quad) -> Self;
    /// Converts `self` into a [`Quad`].
    fn into_quad(self) -> quad;
}

impl Quad for quad {
    #[inline]
    fn from_quad(value: quad) -> Self {
        value
    }

    #[inline]
    fn into_quad(self) -> quad {
        self
    }
}

impl From<u8> for quad {
    #[inline]
    fn from(byte: u8) -> Self {
        assert!(byte <= 0b11);
        match byte {
            0b00 => Self::B00,
            0b01 => Self::B01,
            0b10 => Self::B10,
            0b11 => Self::B11,
            _ => panic!("byte out of bounds for quad"),
        }
    }
}

impl From<quad> for u8 {
    #[inline]
    fn from(quad: quad) -> Self {
        quad as u8
    }
}

/// The raw type of a chunk in the [`BoundedQuadmap`].
///
/// Chunks are the raw entities that store the quads stored in the bounded quad map.
type Chunk = u32;

/// The number of bits used per quad stored in the [`BoundedQuadmap`].
const BITS_PER_QUAD: usize = 2;

/// The number of bits in a single chunk of the [`BoundedQuadmap`].
const CHUNK_LEN: usize = core::mem::size_of::<Chunk>() * 8;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct ChunkIndex {
    value: usize,
}

impl Index for ChunkIndex {
    #[inline]
    fn from_index(index: usize) -> Self {
        ChunkIndex {
            value: index / (CHUNK_LEN / BITS_PER_QUAD),
        }
    }

    #[inline]
    fn into_index(self) -> usize {
        self.value
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct QuadIndex {
    value: usize,
}

impl Index for QuadIndex {
    #[inline]
    fn from_index(index: usize) -> Self {
        Self {
            value: index % (CHUNK_LEN / BITS_PER_QUAD),
        }
    }

    #[inline]
    fn into_index(self) -> usize {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedQuadmap<Idx, T> {
    len: usize,
    chunks: BoundedArray<ChunkIndex, Chunk>,
    marker: PhantomData<fn() -> (Idx, T)>,
}

impl<Idx, T> Default for BoundedQuadmap<Idx, T> {
    fn default() -> Self {
        Self {
            len: 0,
            chunks: BoundedArray::default(),
            marker: Default::default(),
        }
    }
}

impl<Idx, T> BoundedQuadmap<Idx, T>
where
    Idx: Index,
    T: Default,
{
    pub fn with_len(len: usize) -> Self {
        Self {
            len,
            chunks: BoundedArray::with_len(len, |_| Default::default()),
            marker: Default::default(),
        }
    }

    pub fn resize_to_len(&mut self, new_len: usize) {
        self.chunks.resize_with(new_len, Default::default);
        self.len = new_len;
    }
}

impl<Idx, T> BoundedQuadmap<Idx, T>
where
    Idx: Index,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn quad_index_to_mask(index: QuadIndex) -> Chunk {
        (0b11 as Chunk) << (CHUNK_LEN - (BITS_PER_QUAD * (1 + index.into_index())))
    }
}

impl<Idx, T> BoundedQuadmap<Idx, T>
where
    Idx: Index,
    T: Quad,
{
    fn quad_index_to_mask_using(index: QuadIndex, flag: T) -> Chunk {
        (u8::from(flag.into_quad()) as Chunk)
            << (CHUNK_LEN - (BITS_PER_QUAD * (1 + index.into_index())))
    }

    fn split_index(idx: Idx) -> (ChunkIndex, QuadIndex) {
        let raw_index = idx.into_index();
        (
            ChunkIndex::from_index(raw_index),
            QuadIndex::from_index(raw_index),
        )
    }

    #[inline]
    pub fn get(&self, index: Idx) -> Result<T, OutOfBoundsAccess> {
        if index.into_index() >= self.len() {
            return Err(OutOfBoundsAccess)
        }
        let (chunk_idx, quad_idx) = Self::split_index(index);
        let chunk = self.chunks.get(chunk_idx)?;
        let mask = Self::quad_index_to_mask(quad_idx);
        let value =
            (chunk & mask) >> (CHUNK_LEN - (BITS_PER_QUAD * (1 + quad_idx.into_index())));
        debug_assert!(value <= 0b11);
        Ok(T::from_quad(quad::from(value as u8)))
    }

    #[inline]
    pub fn set(&mut self, index: Idx, new_value: T) -> Result<(), OutOfBoundsAccess> {
        if index.into_index() >= self.len() {
            return Err(OutOfBoundsAccess)
        }
        let (chunk_idx, quad_idx) = Self::split_index(index);
        let chunk = self.chunks.get_mut(chunk_idx)?;
        // Empty bits before eventually writing the new bit pattern.
        // If there are bit access patterns that can combine these two steps we should do them instead.
        *chunk &= !Self::quad_index_to_mask(quad_idx);
        *chunk |= Self::quad_index_to_mask_using(quad_idx, new_value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_works() {
        let mut map = <BoundedQuadmap<usize, quad>>::default();
        map.resize_to_len(10);
        map.set(0, quad::B11).unwrap();
        assert_eq!(map.get(0), Ok(quad::B01));
    }
}
