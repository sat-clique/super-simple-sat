pub mod bounded_array;
pub mod bounded_bitmap;
pub mod bounded_heap;
pub mod bounded_map;
pub mod bounded_stack;

pub use self::{
    bounded_array::BoundedArray,
    bounded_bitmap::{
        Bool,
        BoundedBitmap,
    },
    bounded_heap::BoundedHeap,
    bounded_map::BoundedMap,
    bounded_stack::BoundedStack,
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

#[cfg(test)]
mod tests {
    use super::*;

    impl Index for usize {
        fn from_index(index: usize) -> Self {
            index
        }
        fn into_index(self) -> usize {
            self
        }
    }
}
