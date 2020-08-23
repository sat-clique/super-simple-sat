use super::{
    BoundedArray,
    Index,
    OutOfBoundsAccess,
};
use core::marker::PhantomData;

pub trait Bool {
    fn from_bool(value: bool) -> Self;
    fn into_bool(self) -> bool;
}

impl Bool for bool {
    #[inline(always)]
    fn from_bool(value: bool) -> Self {
        value
    }

    #[inline(always)]
    fn into_bool(self) -> bool {
        self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct ChunkIndex {
    value: usize,
}

impl Index for ChunkIndex {
    #[inline]
    fn from_index(index: usize) -> Self {
        ChunkIndex {
            value: index / CHUNK_LEN,
        }
    }

    #[inline]
    fn into_index(self) -> usize {
        self.value
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct BitIndex {
    value: usize,
}

impl Index for BitIndex {
    #[inline]
    fn from_index(index: usize) -> Self {
        Self {
            value: index % CHUNK_LEN,
        }
    }

    #[inline]
    fn into_index(self) -> usize {
        self.value
    }
}

type Chunk = u32;
const CHUNK_LEN: usize = core::mem::size_of::<Chunk>() * 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedBitmap<Idx, T> {
    len: usize,
    chunks: BoundedArray<ChunkIndex, Chunk>,
    marker: PhantomData<fn() -> (Idx, T)>,
}

impl<Idx, T> Default for BoundedBitmap<Idx, T> {
    fn default() -> Self {
        Self {
            chunks: BoundedArray::default(),
            len: 0,
            marker: Default::default(),
        }
    }
}

impl<Idx, T> BoundedBitmap<Idx, T>
where
    Idx: Index,
    T: Bool + Copy,
{
    pub fn from_slice(slice: &[T]) -> Self {
        let len = slice.len();
        let chunks = slice
            .chunks(CHUNK_LEN)
            .map(|chunk| {
                let mut bits = 0;
                for (n, &bit) in chunk.iter().enumerate() {
                    bits |= Self::bit_index_to_mask_iff(BitIndex::from_index(n), bit)
                }
                bits
            })
            .collect::<BoundedArray<ChunkIndex, Chunk>>();
        Self {
            len,
            chunks,
            marker: Default::default(),
        }
    }
}

impl<Idx, T> BoundedBitmap<Idx, T> {
    /// Returns the number of required chunks for the given amount of required quads.
    fn required_chunks(required_quads: usize) -> usize {
        required_quads.saturating_sub(1) / CHUNK_LEN + 1
    }

    pub fn with_len(len: usize) -> Self {
        let len_chunks = Self::required_chunks(len);
        Self {
            chunks: BoundedArray::with_len(len_chunks, |_| Default::default()),
            len,
            marker: Default::default(),
        }
    }

    pub fn resize_to_len(&mut self, new_len: usize) {
        let len_chunks = Self::required_chunks(new_len);
        self.chunks.resize_with(len_chunks, Default::default);
        self.len = new_len;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn bit_index_to_mask(index: BitIndex) -> Chunk {
        0x01 << ((CHUNK_LEN - 1) - index.into_index())
    }
}

impl<Idx, T> BoundedBitmap<Idx, T>
where
    T: Bool,
{
    fn bit_index_to_mask_iff(index: BitIndex, flag: T) -> Chunk {
        (flag.into_bool() as Chunk) << ((CHUNK_LEN - 1) - index.into_index())
    }
}

impl<Idx, T> BoundedBitmap<Idx, T>
where
    Idx: Index,
{
    /// Ensures that the given index is valid for the bounded bit map.
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

impl<Idx, T> BoundedBitmap<Idx, T>
where
    Idx: Index,
    T: Bool,
{
    fn split_index(idx: Idx) -> (ChunkIndex, BitIndex) {
        let raw_index = idx.into_index();
        (
            ChunkIndex::from_index(raw_index),
            BitIndex::from_index(raw_index),
        )
    }

    #[inline]
    pub fn get(&self, index: Idx) -> Result<T, OutOfBoundsAccess> {
        self.ensure_valid_index(index)?;
        let (chunk_idx, bit_idx) = Self::split_index(index);
        let chunk = self
            .chunks
            .get(chunk_idx)
            .expect("unexpected out of bounds chunk");
        let value = chunk & Self::bit_index_to_mask(bit_idx);
        Ok(T::from_bool(value != 0))
    }

    #[inline]
    pub fn set(&mut self, index: Idx, new_value: T) -> Result<(), OutOfBoundsAccess> {
        self.ensure_valid_index(index)?;
        let (chunk_idx, bit_idx) = Self::split_index(index);
        let chunk = self
            .chunks
            .get_mut(chunk_idx)
            .expect("unexpected out of bounds chunk");
        // Empty bits before eventually writing the new bit pattern.
        // If there are bit access patterns that can combine these two steps we should do them instead.
        *chunk &= !Self::bit_index_to_mask(bit_idx);
        *chunk |= Self::bit_index_to_mask_iff(bit_idx, new_value);
        Ok(())
    }

    pub fn iter(&self) -> Iter<Idx, T> {
        Iter::new(self)
    }
}

impl<'a, Idx, T> IntoIterator for &'a BoundedBitmap<Idx, T>
where
    Idx: Index,
    T: Bool,
{
    type Item = T;
    type IntoIter = Iter<'a, Idx, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'a, Idx, T> {
    current: usize,
    bits: &'a BoundedBitmap<Idx, T>,
}

impl<'a, Idx, T> Iter<'a, Idx, T> {
    fn new(bitmap: &'a BoundedBitmap<Idx, T>) -> Self {
        Self {
            current: 0,
            bits: bitmap,
        }
    }
}

impl<'a, Idx, T> Iterator for Iter<'a, Idx, T>
where
    Idx: Index,
    T: Bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.bits.len() {
            return None
        }
        match self.bits.get(Idx::from_index(self.current)) {
            Ok(value) => {
                self.current += 1;
                Some(value)
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_works() {
        let map = <BoundedBitmap<usize, bool>>::default();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn with_len_works() {
        let map = <BoundedBitmap<usize, bool>>::with_len(10);
        assert_eq!(map.len(), 10);
        assert!(!map.is_empty());
        for i in 0..10 {
            assert_eq!(map.get(i), Ok(false));
        }
    }

    #[test]
    fn set_works() {
        let mut map = <BoundedBitmap<usize, bool>>::default();
        map.resize_to_len(3);
        assert_eq!(map.get(0), Ok(false));
        map.set(0, true).unwrap();
        assert_eq!(map.get(0), Ok(true));
        map.set(0, false).unwrap();
        assert_eq!(map.get(0), Ok(false));
    }

    #[test]
    fn get_out_of_bounds_fails() {
        let map = <BoundedBitmap<usize, bool>>::with_len(3);
        assert!(map.get(0).is_ok());
        assert!(map.get(1).is_ok());
        assert!(map.get(2).is_ok());
        assert_eq!(map.get(3), Err(OutOfBoundsAccess));
    }

    #[test]
    fn set_out_of_bounds_fails() {
        let mut map = <BoundedBitmap<usize, bool>>::with_len(3);
        assert!(map.set(0, false).is_ok());
        assert!(map.set(1, true).is_ok());
        assert!(map.set(2, false).is_ok());
        assert_eq!(map.set(3, true), Err(OutOfBoundsAccess));
    }

    #[test]
    fn set_all_multiword_works() {
        let len = 100;
        let mut map = <BoundedBitmap<usize, bool>>::with_len(len);
        // false -> true
        for i in 0..len {
            assert_eq!(map.get(i), Ok(false));
            map.set(i, true).unwrap();
            assert_eq!(map.get(i), Ok(true));
        }
        // true -> false
        for i in 0..len {
            assert_eq!(map.get(i), Ok(true));
            map.set(i, false).unwrap();
            assert_eq!(map.get(i), Ok(false));
        }
    }
}
