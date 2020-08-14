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
#[derive(Debug, Clone)]
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

impl<K, W> Default for BoundedHeap<K, W>
where
    K: Index,
    W: Default,
{
    fn default() -> Self {
        Self {
            len: 0,
            heap: BoundedArray::default(),
            positions: BoundedArray::default(),
            weights: BoundedArray::default(),
        }
    }
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

    /// Returns `true` if the heap property is satisfied for all elements in the bounded heap.
    ///
    /// # Note
    ///
    /// The heap property is that the weight of parent nodes is always greater than or equal
    /// to the weight of their children.
    ///
    /// This is a test-only API and generally not available.
    #[cfg(test)]
    fn satisfies_heap_property(&self) -> bool {
        for i in 1..self.len() {
            let child = HeapPosition::from_index(i);
            let parent = child.parent().expect("encountered missing parent");
            let child_key = self
                .heap
                .get(child)
                .expect("encountered missing child heap entry");
            let parent_key = self
                .heap
                .get(parent)
                .expect("encountered missing parent heap entry");
            if self.cmp_weights(*parent_key, *child_key).expect(
                "encountered error upon comparing parent and right child weights",
            ) != Ordering::Greater
            {
                return false
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_heap_is_marked_as_empty() {
        let mut heap = <BoundedHeap<usize, i32>>::default();
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.capacity(), 0);
        assert!(heap.is_empty());
        heap.increase_capacity_to(10).unwrap();
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.capacity(), 10);
        assert!(heap.is_empty());
    }

    #[test]
    fn empty_heap_contains_no_elements() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.increase_capacity_to(size).unwrap();
        for i in 0..size {
            assert_eq!(heap.contains(i), Ok(false));
        }
    }

    #[test]
    fn single_element_heap_contains_exactly_one_element() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.increase_capacity_to(size).unwrap();
        heap.push_or_update(5, 42).unwrap();
        assert!(!heap.is_empty());
        assert_eq!(heap.len(), 1);
        for i in 0..10 {
            assert_eq!(heap.contains(i), Ok(i == 5), "{} unexpectedly contained", i);
        }
    }

    #[test]
    fn no_duplicate_elements_upon_double_insertion() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.increase_capacity_to(size).unwrap();
        heap.push_or_update(5, 42).unwrap();
        heap.push_or_update(5, 42).unwrap();
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.pop(), Some((5, 42)));
        assert!(heap.is_empty());
    }

    #[test]
    fn single_element_heap_is_empty_after_pop() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.increase_capacity_to(size).unwrap();
        heap.push_or_update(5, 42).unwrap();
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.pop(), Some((5, 42)));
        assert!(heap.is_empty());
    }

    #[test]
    fn satisfies_heap_property_after_insertion() {
        let test_weights = [3, 9, 1, -5, -10, -9, 10, 0, -1, 7];
        let size = test_weights.len();
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.increase_capacity_to(size).unwrap();
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, w).unwrap();
        }
        assert_eq!(heap.len(), test_weights.len());
        assert!(heap.satisfies_heap_property());
    }

    #[test]
    fn heap_can_be_filled_to_max() {
        let len = 10;
        let mut heap = BoundedHeap::default();
        heap.increase_capacity_to(len).unwrap();
        for (k, w) in (0..len).map(|i| i * 10).enumerate() {
            heap.push_or_update(k, w).unwrap();
        }
        assert_eq!(heap.len(), 10);
    }

    #[test]
    fn out_of_bounds_key_is_rejected() {
        let len = 10;
        let mut heap = BoundedHeap::default();
        heap.increase_capacity_to(len).unwrap();
        assert_eq!(heap.push_or_update(10, 42), Err(Error::Bounded(BoundedError::OutOfBoundsAccess)));
    }

    #[test]
    fn has_descending_removal_sequence() {
        let test_weights = [3, 9, 1, -5, -10, -9, 10, 0, -1, 7];
        let len = test_weights.len();
        let mut heap = BoundedHeap::default();
        heap.increase_capacity_to(len).unwrap();
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, w).unwrap();
        }
        assert!(heap.satisfies_heap_property());
        let mut removed_sequence = Vec::new();
        while let Some((k, w)) = heap.pop() {
            removed_sequence.push(w);
            assert!(heap.satisfies_heap_property(), "heap property NOT satisfied after popping key {}", k);
        }
        let expected_sequence = {
            let mut weights = test_weights.to_vec();
            weights.sort_by_key(|k| core::cmp::Reverse(*k));
            weights
        };
        assert_eq!(removed_sequence, expected_sequence);
        assert!(heap.is_empty());
    }

    #[test]
    fn push_pop_sequence_works() {
        let test_weights = [3, 9, 1, -5];
        let len = test_weights.len();
        let mut heap = BoundedHeap::default();
        heap.increase_capacity_to(10).unwrap();
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, w).unwrap();
        }
        assert_eq!(heap.pop(), Some((1, 9)));
        assert_eq!(heap.pop(), Some((0, 3)));
        heap.push_or_update(len, 2).unwrap();
        assert_eq!(heap.pop(), Some((len, 2)));
        heap.push_or_update(len + 1, -3).unwrap();
        assert_eq!(heap.pop(), Some((2, 1)));
        assert_eq!(heap.pop(), Some((len + 1, -3)));
        assert_eq!(heap.pop(), Some((3, -5)));
    }

    #[test]
    fn heap_can_be_resized() {
        let test_weights = [10, 30, 20];
        let len = test_weights.len();
        let mut heap = BoundedHeap::default();
        heap.increase_capacity_to(len).unwrap();
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, w).unwrap();
        }
        assert_eq!(heap.len(), len);
        assert_eq!(heap.push_or_update(len, 40), Err(Error::Bounded(BoundedError::OutOfBoundsAccess)));
        heap.increase_capacity_to(len + 1).unwrap();
        heap.push_or_update(len, 40).unwrap();
        assert_eq!(heap.len(), len + 1);
        assert_eq!(heap.pop(), Some((len, 40)));
    }
}
