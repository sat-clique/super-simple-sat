use super::Error as BoundedError;
use crate::{
    utils::{
        BoundedArray,
        Index,
    },
    Error,
};
use core::{
    cmp::Ordering,
    mem,
    num::NonZeroUsize,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct HeapPosition(NonZeroUsize);

impl HeapPosition {
    /// Returns the root heap position.
    pub fn root() -> Self {
        Self::from_index(0)
    }

    /// Returns the heap position of the left child in relation to self.
    pub fn left_child(self) -> Self {
        Self::from_index(self.into_index() * 2 + 1)
    }

    /// Returns the heap position of the right child in relation to self.
    pub fn right_child(self) -> Self {
        Self::from_index(self.into_index() * 2 + 2)
    }

    /// Returns `true` if the heap position refers to the root, e.g. maximum element.
    fn is_root(self) -> bool {
        self.into_index() == 0
    }

    /// Returns the heap position of the parent in relation to self.
    ///
    /// Returns `None` if self is the root.
    pub fn parent(self) -> Option<Self> {
        if self.is_root() {
            return None
        }
        Some(Self::from_index((self.into_index() - 1) / 2))
    }
}

impl Index for HeapPosition {
    fn from_index(index: usize) -> Self {
        Self(
            NonZeroUsize::new(index.wrapping_add(1))
                .expect("encountered invalid heap position index"),
        )
    }

    fn into_index(self) -> usize {
        self.0.get().wrapping_sub(1)
    }
}

/// A bounded binary max-heap that supports update of key weights.
#[derive(Debug, Default, Clone)]
pub struct BoundedHeap<K, W> {
    /// The number of current elements.
    len: usize,
    /// The actual heap storing the keys according to heap properties.
    ///
    /// The keys are used to refer to their underlying weights via the
    /// associated `weights` array.
    ///
    /// If the heap position of a key changes the `positions` array needs
    /// to be updated.
    heap: BoundedArray<HeapPosition, K>,
    /// The current index in the `heap` array for every key.
    positions: BoundedArray<K, Option<HeapPosition>>,
    /// The weight for every key.
    weights: BoundedArray<K, W>,
}

impl<K, W> BoundedHeap<K, W>
where
    K: Default + Index + Eq,
    W: Default + Ord + Copy,
{
    /// Returns the number of elements stored in the bounded heap.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the bounded heap is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of the bounded heap.
    pub fn capacity(&self) -> usize {
        self.weights.len()
    }

    /// Returns `true` if the element associated with the given key is contained.
    pub fn contains(&self, key: K) -> Result<bool, BoundedError> {
        Ok(self.positions.get(key)?.is_some())
    }

    /// Returns the index of the left child given the heap position.
    ///
    /// Returns `None` if the child index would be out of bounds.
    fn left_child(&self, position: HeapPosition) -> Option<HeapPosition> {
        let child = position.left_child();
        if child.into_index() >= self.len() {
            return None
        }
        Some(child)
    }

    /// Returns the index of the right child given the heap position.
    ///
    /// Returns `None` if the child index would be out of bounds.
    fn right_child(&self, position: HeapPosition) -> Option<HeapPosition> {
        let child = position.right_child();
        if child.into_index() >= self.len() {
            return None
        }
        Some(child)
    }

    /// Increases the length of the bounded heap to the new length.
    pub fn increase_capacity_to(&mut self, new_cap: usize) -> Result<(), Error> {
        self.heap.increase_len_to(new_cap)?;
        self.positions.increase_len_to(new_cap)?;
        self.weights.increase_len_to(new_cap)?;
        Ok(())
    }

    /// Returns the heap position of the given key.
    ///
    /// # Errors
    ///
    /// - If the given key index is out of bounds for the bounded heap.
    /// - If the key does not currently have a heap position.
    fn heap_position(&self, key: K) -> Result<HeapPosition, BoundedError> {
        self.positions
            .get(key)?
            .as_ref()
            .copied()
            .ok_or_else(|| BoundedError::OutOfBoundsAccess)
    }

    /// Pushes the key to the heap.
    ///
    /// This increases the length of the bounded heap and updates the positions array.
    ///
    /// # Panics
    ///
    /// If the key is already contained in the heap.
    fn push_heap_position(&mut self, key: K) -> Result<HeapPosition, BoundedError> {
        assert!(
            !self.contains(key)?,
            "encountered already contained key upon push"
        );
        let last_position = HeapPosition::from_index(self.len);
        self.positions.update(key, Some(last_position))?;
        self.heap.update(last_position, key)?;
        self.len += 1;
        Ok(last_position)
    }

    /// Inserts a new key/weight pair into the heap or updates the weight of an existing key.
    ///
    /// Increases the length of the heap if the key was not contained before.
    ///
    /// # Errors
    ///
    /// If the key index is out of bounds.
    pub fn push_or_update(&mut self, key: K, new_weight: W) -> Result<(), Error> {
        let already_contained = self.contains(key)?;
        if !already_contained {
            self.push_heap_position(key)?;
        }
        let is_weight_increased = {
            let old_weight = mem::replace(self.weights.get_mut(key)?, new_weight);
            !already_contained || old_weight <= new_weight
        };
        let position = self.heap_position(key)?;
        match is_weight_increased {
            true => self.sift_up(position)?,
            false => self.sift_down(position)?,
        }
        Ok(())
    }

    /// Compares the weights of the given keys.
    ///
    /// # Errors
    ///
    /// If any of the given keys is out of bounds for the bounded heap.
    fn cmp_weights(&self, lhs: K, rhs: K) -> Result<Ordering, Error> {
        if lhs == rhs {
            return Ok(Ordering::Equal)
        }
        let lhs_weight = self.weights.get(lhs)?;
        let rhs_weight = self.weights.get(rhs)?;
        Ok(lhs_weight.cmp(rhs_weight))
    }

    /// Adjusts the heap ordering for the given pivot element.
    ///
    /// # Note
    ///
    /// Used if the weight of the pivot element has been increased or after
    /// a new key weight pair has been inserted into the heap.
    fn sift_up(&mut self, pivot: HeapPosition) -> Result<(), Error> {
        let pivot_key = *self.heap.get(pivot)?;
        let mut cursor = pivot;
        'perculate: while let Some(parent) = cursor.parent() {
            let parent_key = *self.heap.get(parent)?;
            match self.cmp_weights(pivot_key, parent_key)? {
                Ordering::Greater => {
                    // Child is greater than the current parent -> move down the parent.
                    self.heap.update(cursor, parent_key)?;
                    self.positions.update(parent_key, Some(cursor))?;
                    cursor = parent;
                }
                Ordering::Equal | Ordering::Less => break 'perculate,
            }
        }
        self.heap.update(cursor, pivot_key)?;
        self.positions.update(pivot_key, Some(cursor))?;
        Ok(())
    }

    /// Adjusts the heap ordering for the given pivot element.
    ///
    /// # Note
    ///
    /// Used of the weight of the pivot element has been decreased or the root
    /// element has been popped.
    fn sift_down(&mut self, pivot: HeapPosition) -> Result<(), Error> {
        let pivot_key = *self.heap.get(pivot)?;
        let mut cursor = pivot;
        'perculate: while let Some(left_child) = self.left_child(cursor) {
            let right_child = self.right_child(cursor);
            let max_child = match right_child {
                Some(right_child) => {
                    let left_child_key = *self.heap.get(left_child)?;
                    let right_child_key = *self.heap.get(right_child)?;
                    match self.cmp_weights(left_child_key, right_child_key)? {
                        Ordering::Less | Ordering::Equal => right_child,
                        Ordering::Greater => left_child,
                    }
                }
                None => left_child,
            };
            let max_child_key = *self.heap.get(max_child)?;
            if self.cmp_weights(pivot_key, max_child_key)? == Ordering::Less {
                // Child is greater than element -> move it upwards.
                self.heap.update(cursor, max_child_key)?;
                self.positions.update(max_child_key, Some(cursor))?;
                cursor = max_child;
            } else {
                break 'perculate
            }
        }
        self.heap.update(cursor, pivot_key)?;
        self.positions.update(pivot_key, Some(cursor))?;
        Ok(())
    }

    /// Returns a shared reference to the current maximum key and its weight.
    ///
    /// This does not pop the maximum element from the bounded heap.
    pub fn peek(&self) -> Option<(&K, &W)> {
        if self.is_empty() {
            return None
        }
        let key = self
            .heap
            .get(HeapPosition::root())
            .expect("encountered unexpected empty heap array");
        let weight = self
            .weights
            .get(*key)
            .expect("encountered invalid root key");
        Some((key, weight))
    }

    /// Pops the current maximum key and its weight from the bounded heap.
    pub fn pop(&mut self) -> Option<(K, W)> {
        if self.is_empty() {
            return None
        }
        let key = *self
            .heap
            .get(HeapPosition::root())
            .expect("encountered unexpected empty heap array");
        self.positions
            .update(key, None)
            .expect("encountered invalid root key");
        let weight = *self.weights.get(key).expect("encountered invalid root key");
        if self.len == 1 {
            // No need to adjust heap properties.
            self.len = 0;
        } else {
            // Replace root with the last element of the heap.
            let new_root = *self
                .heap
                .get(HeapPosition::from_index(self.len - 1))
                .expect("unexpected missing last element in heap");
            self.heap
                .update(HeapPosition::root(), new_root)
                .expect("encountered error upon heap update of new root");
            self.positions
                .update(new_root, Some(HeapPosition::root()))
                .expect("encountered unexpected error upon positions heap update");
            self.len -= 1;
            self.sift_down(HeapPosition::root())
                .expect("encountered error upon sifting down new root in heap");
        }
        Some((key, weight))
    }
}
