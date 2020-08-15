use super::{
    BoundedArray,
    Error,
    Index,
};
use core::marker::PhantomData;

pub trait Bool {
    fn from_bool(value: bool) -> Self;
    fn into_bool(self) -> bool;
}

impl Bool for bool {
    fn from_bool(value: bool) -> Self {
        value
    }

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
    fn from_index(index: usize) -> Self {
        ChunkIndex {
            value: index / CHUNK_LEN,
        }
    }

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
    fn from_index(index: usize) -> Self {
        BitIndex {
            value: index % CHUNK_LEN,
        }
    }

    fn into_index(self) -> usize {
        self.value
    }
}

type Chunk = u32;
const CHUNK_LEN: usize = core::mem::size_of::<Chunk>() * 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedBitmap<Idx, T> {
    chunks: BoundedArray<ChunkIndex, Chunk>,
    len: usize,
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
    pub fn with_len(len: usize) -> Self {
        Self {
            chunks: BoundedArray::with_len(len, |_| Default::default()),
            len,
            marker: Default::default(),
        }
    }

    pub fn resize_with(&mut self, new_len: usize) {
        self.chunks
            .resize_with((new_len / CHUNK_LEN) + 1, Default::default);
        self.len = new_len;
    }

    fn bit_index_to_mask(index: BitIndex) -> Chunk {
        0x01 << ((CHUNK_LEN - 1) - index.into_index())
    }

    pub fn len(&self) -> usize {
        self.len
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
    T: Bool,
{
    fn split_index(idx: Idx) -> (ChunkIndex, BitIndex) {
        let raw_index = idx.into_index();
        (
            ChunkIndex::from_index(raw_index),
            BitIndex::from_index(raw_index),
        )
    }

    pub fn get(&self, index: Idx) -> Result<T, Error> {
        let (chunk_idx, bit_idx) = Self::split_index(index);
        let chunk = self.chunks.get(chunk_idx)?;
        let value = chunk & Self::bit_index_to_mask(bit_idx);
        Ok(T::from_bool(value != 0))
    }

    pub fn set(&mut self, index: Idx, new_value: T) -> Result<(), Error> {
        let new_value = new_value.into_bool();
        let (chunk_idx, bit_idx) = Self::split_index(index);
        let chunk = self.chunks.get_mut(chunk_idx)?;
        match new_value {
            true => {
                *chunk |= Self::bit_index_to_mask(bit_idx);
            }
            false => {
                *chunk &= !Self::bit_index_to_mask(bit_idx);
            }
        }
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
