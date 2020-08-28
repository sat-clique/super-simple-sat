#![forbid(unsafe_code)]
#![no_std]

extern crate alloc;

pub mod bounded_array;
pub mod bounded_bitmap;
pub mod bounded_heap;
pub mod bounded_map;
pub mod bounded_stack;
pub mod bounded_quadmap;

pub use self::{
    bounded_array::BoundedArray,
    bounded_bitmap::{
        Bool,
        BoundedBitmap,
    },
    bounded_heap::BoundedHeap,
    bounded_map::BoundedMap,
    bounded_stack::BoundedStack,
    bounded_quadmap::{
        BoundedQuadmap,
        Quad,
    },
};

/// Errors that may occure when operating on a bounded data structure.
#[derive(Debug, PartialEq, Eq)]
pub struct OutOfBoundsAccess;

/// Index types that may be used as keys for the bounded map.
pub trait Index: Copy + Clone {
    /// Creates a new key from the given index.
    fn from_index(index: usize) -> Self;
    /// Returns the index from the given key.
    fn into_index(self) -> usize;
}

impl Index for usize {
    #[inline]
    fn from_index(index: usize) -> Self {
        index
    }

    #[inline]
    fn into_index(self) -> usize {
        self
    }
}
