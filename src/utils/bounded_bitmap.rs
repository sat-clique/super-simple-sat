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

#[derive(Debug, Copy, Clone)]
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

type Bits32 = u32;
const CHUNK_LEN: usize = core::mem::size_of::<Bits32>() * 8;

pub struct BoundedBitmap<Idx, T> {
    chunks: BoundedArray<ChunkIndex, Bits32>,
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
    T: Bool + Copy,
{
    pub fn from_slice(slice: &[T]) -> Self {
        let len = slice.len();
        let chunks = slice
            .chunks(CHUNK_LEN)
            .map(|chunk| {
                let mut bits = 0;
                for (n, &bit) in chunk.iter().enumerate() {
                    bits |= (bit.into_bool() as u32) << (31 - n);
                }
                bits
            })
            .collect::<BoundedArray<ChunkIndex, Bits32>>();
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
            chunks: BoundedArray::with_len(len),
            len,
            marker: Default::default(),
        }
    }

    pub fn increase_len(&mut self, new_len: usize) -> Result<(), Error> {
        self.chunks.increase_to_capacity((new_len / 32) + 1)?;
        self.len = new_len;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
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
        let bit_idx = bit_idx.into_index();
        let value = (chunk >> (31 - bit_idx)) & 0x01 != 0;
        Ok(T::from_bool(value))
    }

    pub fn set(&mut self, index: Idx, new_value: T) -> Result<(), Error> {
        let new_value = new_value.into_bool();
        let (chunk_idx, bit_idx) = Self::split_index(index);
        let chunk = self.chunks.get_mut(chunk_idx)?;
        let bit_idx = bit_idx.into_index();
        match new_value {
            true => {
                *chunk |= 0x01 << (31 - bit_idx);
            }
            false => {
                *chunk &= !(0x01 << (31 - bit_idx));
            }
        }
        Ok(())
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
        todo!()
    }
}

pub struct Iter<'a, Idx, T> {
    current: usize,
    bits: &'a BoundedBitmap<Idx, T>,
}

impl<'a, Idx, T> Iterator for Iter<'a, Idx, T>
where
    Idx: Index,
    T: Bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.bits.get(Idx::from_index(self.current)) {
            Ok(value) => {
                self.current += 1;
                Some(value)
            }
            Err(_) => None,
        }
    }
}
