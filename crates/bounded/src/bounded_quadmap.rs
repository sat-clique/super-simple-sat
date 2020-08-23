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

/// An internal chunk index within the bounded quad map.
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

/// An internal quad index within a chunk of the bounded quad map.
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
    /// Returns the number of required chunks for the given amount of required quads.
    fn required_chunks(required_quads: usize) -> usize {
        required_quads.saturating_sub(1) * BITS_PER_QUAD / CHUNK_LEN + 1
    }

    /// Creates a new bounded quad map with the given length.
    ///
    /// All elements are initialized with their default values.
    pub fn with_len(len: usize) -> Self {
        let len_chunks = Self::required_chunks(len);
        Self {
            len,
            chunks: BoundedArray::with_len(len_chunks, |_| Default::default()),
            marker: Default::default(),
        }
    }

    /// Resizes the bounded quad map to the new length.
    ///
    /// Shrinks the size if the new length is lower than the current length.
    /// If the length is increased all new elements are initialized with their
    /// default values.
    pub fn resize_to_len(&mut self, new_len: usize) {
        let len_chunks = Self::required_chunks(new_len);
        self.chunks.resize_with(len_chunks, Default::default);
        self.len = new_len;
    }
}

impl<Idx, T> BoundedQuadmap<Idx, T>
where
    Idx: Index,
{
    /// Returns the number of quads that are stored in the bounded quad map.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the bounded quad map is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the bit mask for the quad at the given index.
    ///
    /// # Note
    ///
    /// The bit mask shadows all but the necessary bits for the quad to exact the quad
    /// information that the given index refers to.
    fn quad_index_to_mask(index: QuadIndex) -> Chunk {
        (0b11 as Chunk) << (CHUNK_LEN - (BITS_PER_QUAD * (1 + index.into_index())))
    }

    /// Ensures that the given index is valid for the bounded quad map.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds.
    fn ensure_valid_index(&self, index: Idx) -> Result<usize, OutOfBoundsAccess> {
        let index = index.into_index();
        if index >= self.len() {
            return Err(OutOfBoundsAccess)
        }
        Ok(index)
    }
}

impl<Idx, T> BoundedQuadmap<Idx, T>
where
    Idx: Index,
    T: Quad,
{
    /// Returns the bit mask for the quad at the given index using another quad.
    ///
    /// # Note
    ///
    /// The bit mask shadows all but the necessary bits for the quad to exact the quad
    /// information that the given index refers to.
    /// The given quad's bit representation will be used at the bitmask for shadowing.
    fn quad_index_to_mask_using(index: QuadIndex, flag: T) -> Chunk {
        (u8::from(flag.into_quad()) as Chunk)
            << (CHUNK_LEN - (BITS_PER_QUAD * (1 + index.into_index())))
    }

    /// Splits the given index into chunk and quad indices.
    fn split_index(idx: Idx) -> (ChunkIndex, QuadIndex) {
        let raw_index = idx.into_index();
        (
            ChunkIndex::from_index(raw_index),
            QuadIndex::from_index(raw_index),
        )
    }

    /// Returns the quad at the given index.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds for the bounded array.
    #[inline]
    pub fn get(&self, index: Idx) -> Result<T, OutOfBoundsAccess> {
        self.ensure_valid_index(index)?;
        let (chunk_idx, quad_idx) = Self::split_index(index);
        let chunk = self
            .chunks
            .get(chunk_idx)
            .expect("unexpected out of bounds chunk");
        let mask = Self::quad_index_to_mask(quad_idx);
        let shift_len = CHUNK_LEN - (BITS_PER_QUAD * (1 + quad_idx.into_index()));
        let value = (chunk & mask) >> shift_len;
        debug_assert!(value <= 0b11);
        Ok(T::from_quad(quad::from(value as u8)))
    }

    /// Sets the value of the quad at the given index.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds for the bounded array.
    #[inline]
    pub fn set(&mut self, index: Idx, new_value: T) -> Result<(), OutOfBoundsAccess> {
        self.ensure_valid_index(index)?;
        let (chunk_idx, quad_idx) = Self::split_index(index);
        let chunk = self
            .chunks
            .get_mut(chunk_idx)
            .expect("unexpected out of bounds chunk");
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
    fn default_works() {
        let map = <BoundedQuadmap<usize, quad>>::default();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn with_len_works() {
        let map = <BoundedQuadmap<usize, quad>>::with_len(10);
        assert_eq!(map.len(), 10);
        assert!(!map.is_empty());
        for i in 0..10 {
            assert_eq!(map.get(i), Ok(quad::B00));
        }
    }

    #[test]
    fn set_works() {
        let mut map = <BoundedQuadmap<usize, quad>>::default();
        map.resize_to_len(10);
        assert_eq!(map.get(0), Ok(quad::B00));
        map.set(0, quad::B01).unwrap();
        assert_eq!(map.get(0), Ok(quad::B01));
        map.set(0, quad::B10).unwrap();
        assert_eq!(map.get(0), Ok(quad::B10));
        map.set(0, quad::B11).unwrap();
        assert_eq!(map.get(0), Ok(quad::B11));
    }

    #[test]
    fn get_out_of_bounds_fails() {
        let map = <BoundedQuadmap<usize, quad>>::with_len(3);
        assert!(map.get(0).is_ok());
        assert!(map.get(1).is_ok());
        assert!(map.get(2).is_ok());
        assert_eq!(map.get(3), Err(OutOfBoundsAccess));
    }

    #[test]
    fn set_out_of_bounds_fails() {
        let mut map = <BoundedQuadmap<usize, quad>>::with_len(3);
        assert!(map.set(0, quad::B01).is_ok());
        assert!(map.set(1, quad::B10).is_ok());
        assert!(map.set(2, quad::B11).is_ok());
        assert_eq!(map.set(3, quad::B11), Err(OutOfBoundsAccess));
    }

    #[test]
    fn set_all_multiword_works() {
        let len = 100;
        let mut map = <BoundedQuadmap<usize, quad>>::with_len(len);
        for i in 0..len {
            assert_eq!(map.get(i), Ok(quad::B00));
            let set_to = match i % 4 {
                0 => quad::B00,
                1 => quad::B01,
                2 => quad::B10,
                3 => quad::B11,
                _ => unreachable!(),
            };
            map.set(i, set_to).unwrap();
            assert_eq!(map.get(i), Ok(set_to));
        }
    }
}
