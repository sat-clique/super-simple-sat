use super::OutOfBoundsAccess;
use crate::{
    utils::{
        BoundedArray,
        Index,
    },
};
use core::{
    cmp::Ordering,
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

/// A bounded binary max-heap that supports update of key priorities.
#[derive(Debug, Clone)]
pub struct BoundedHeap<K, W> {
    /// The number of current elements.
    len: usize,
    /// The actual heap storing the keys according to heap properties.
    ///
    /// The keys are used to refer to their underlying priorities via the
    /// associated `priorities` array.
    ///
    /// If the heap position of a key changes the `positions` array needs
    /// to be updated.
    heap: BoundedArray<HeapPosition, K>,
    /// The current index in the `heap` array for every key.
    positions: BoundedArray<K, Option<HeapPosition>>,
    /// The priority for every key.
    priorities: BoundedArray<K, W>,
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
            priorities: BoundedArray::default(),
        }
    }
}

impl<K, W> BoundedHeap<K, W>
where
    K: Index + Eq,
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
        self.priorities.len()
    }

    /// Returns `Ok` if the key is within bounds for the bounded heap.
    ///
    /// # Errors
    ///
    /// If the key is not within valid bounds of the bounded heap.
    fn ensure_valid_key(&self, key: K) -> Result<(), OutOfBoundsAccess> {
        if key.into_index() >= self.capacity() {
            return Err(OutOfBoundsAccess)
        }
        Ok(())
    }

    /// Returns `true` if the element associated with the given key is contained.
    pub fn contains(&self, key: K) -> Result<bool, OutOfBoundsAccess> {
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

    /// Resizes the capacity of the bounded heap.
    pub fn resize_capacity(&mut self, new_cap: usize) {
        self.heap.resize_with(new_cap, || K::from_index(0));
        self.positions.resize_with(new_cap, Default::default);
        self.priorities.resize_with(new_cap, Default::default);
    }

    /// Pushes the key to the heap.
    ///
    /// This increases the length of the bounded heap and updates the positions array.
    ///
    /// # Panics
    ///
    /// If the key is already contained in the heap.
    ///
    /// # Errors
    ///
    /// If the key is out of bounds for the bounded heap.
    fn push_heap_position(&mut self, key: K) -> HeapPosition {
        assert!(
            !self
                .contains(key)
                .expect("unexpected out of bounds key (push heap position)"),
            "encountered already contained key upon push"
        );
        let last_position = HeapPosition::from_index(self.len);
        self.update_position(key, last_position);
        self.len += 1;
        last_position
    }

    /// Inserts a new key/priority pair into the heap or updates the priority of an existing key.
    ///
    /// Increases the length of the heap if the key was not contained before.
    ///
    /// # Note
    ///
    /// Restore a key and its old priority by using the identify function for `eval_new_priority`.
    ///
    /// # Errors
    ///
    /// If the key index is out of bounds.
    pub fn push_or_update<F>(
        &mut self,
        key: K,
        eval_new_priority: F,
    ) -> Result<(), OutOfBoundsAccess>
    where
        F: FnOnce(W) -> W,
    {
        self.ensure_valid_key(key)?;
        let already_contained = self
            .contains(key)
            .expect("unexpected invalid key (contains)");
        if !already_contained {
            self.push_heap_position(key);
        }
        let is_priority_increased = {
            let old_priority = self.get_priority(key);
            let new_priority = eval_new_priority(old_priority);
            self.priorities
                .update(key, new_priority)
                .expect("unexpected invalid key (update priority)");
            !already_contained || old_priority <= new_priority
        };
        let position = self
            .get_position(key)
            .expect("unexpected uncontained key (push or update)");
        match is_priority_increased {
            true => self.sift_up(position),
            false => self.sift_down(position),
        }
        Ok(())
    }

    /// Updates the priority of a key.
    ///
    /// If the key is contained in the heap this will also adjust the heap structure.
    /// In case the key is not contained the priority will still be adjusted. This is
    /// useful in case the key is going to be restored in the future to its former
    /// priority.
    ///
    /// # Errors
    ///
    /// If the given key is out of bounds for the bounded heap.
    pub fn update_priority<F>(
        &mut self,
        key: K,
        eval_new_priority: F,
    ) -> Result<(), OutOfBoundsAccess>
    where
        F: FnOnce(W) -> W,
    {
        self.ensure_valid_key(key)?;
        let is_priority_increased = {
            let old_priority = self.get_priority(key);
            let new_priority = eval_new_priority(old_priority);
            self.priorities
                .update(key, new_priority)
                .expect("unexpected out of bounds key (priority update)");
            old_priority <= new_priority
        };
        let position = self
            .get_position(key)
            .expect("unexpected uncontained key (update priority)");
        if self
            .contains(key)
            .expect("encountered unexpected invalid key (contains query)")
        {
            match is_priority_increased {
                true => self.sift_up(position),
                false => self.sift_down(position),
            }
        }
        Ok(())
    }

    /// Transforms the priorities of all valid keys using the given closure.
    ///
    /// The heap properties must be satisfied after the transformation.
    ///
    /// # Note
    ///
    /// This also transforms priorities of keys that are uncontained in the heap.
    ///
    /// # Example
    ///
    /// An example use case is to divide the priorities of all keys in the heap
    /// by some positive number which conserves the heap property for all keys.
    ///
    /// # Panics
    ///
    /// If the heap properties are not satisfied after the transformation.
    ///
    /// This panics instead of returning an error since the heap is in an
    /// unresolvable inconsistent state if a call to this method invalidates
    /// its heap properties.
    pub fn transform_priorities<F>(&mut self, mut new_priority_eval: F)
    where
        F: FnMut(W) -> W,
    {
        for priority in &mut self.priorities {
            *priority = new_priority_eval(*priority);
        }
        assert!(self.satisfies_heap_property());
    }

    /// Compares the priorities of the given keys.
    ///
    /// # Panics
    ///
    /// If any of the given keys is out of bounds for the bounded heap.
    fn cmp_priorities(&self, lhs: K, rhs: K) -> Ordering {
        if lhs == rhs {
            return Ordering::Equal
        }
        let lhs_priority = self.get_priority(lhs);
        let rhs_priority = self.get_priority(rhs);
        lhs_priority.cmp(&rhs_priority)
    }

    /// Adjusts the heap ordering for the given pivot element.
    ///
    /// # Note
    ///
    /// Used if the priority of the pivot element has been increased or after
    /// a new key priority pair has been inserted into the heap.
    ///
    /// # Panics
    ///
    /// If the given heap position is out of bounds for the bounded heap.
    fn sift_up(&mut self, pivot: HeapPosition) {
        assert!(
            pivot.into_index() < self.len(),
            "unexpected out of bounds heap position (sift-up). \
             position is {:?} but heap len is {}",
            pivot,
            self.len(),
        );
        let pivot_key = self.heap_entry(pivot);
        let mut cursor = pivot;
        'perculate: while let Some(parent) = cursor.parent() {
            let parent_key = self.heap_entry(parent);
            match self.cmp_priorities(pivot_key, parent_key) {
                Ordering::Greater => {
                    // Child is greater than the current parent -> move down the parent.
                    self.update_position(parent_key, cursor);
                    cursor = parent;
                }
                Ordering::Equal | Ordering::Less => break 'perculate,
            }
        }
        self.update_position(pivot_key, cursor);
    }

    /// Adjusts the heap ordering for the given pivot element.
    ///
    /// # Note
    ///
    /// Used of the priority of the pivot element has been decreased or the root
    /// element has been popped.
    ///
    /// # Panics
    ///
    /// If the given heap position is out of bounds for the bounded heap.
    fn sift_down(&mut self, pivot: HeapPosition) {
        assert!(
            pivot.into_index() < self.len(),
            "unexpected out of bounds heap position (sift-down). \
             position is {:?} but heap len is {}",
            pivot,
            self.len(),
        );
        let pivot_key = self.heap_entry(pivot);
        let mut cursor = pivot;
        'perculate: while let Some(left_child) = self.left_child(cursor) {
            let right_child = self.right_child(cursor);
            let max_child = match right_child {
                Some(right_child) => {
                    let left_child_key = self.heap_entry(left_child);
                    let right_child_key = self.heap_entry(right_child);
                    match self.cmp_priorities(left_child_key, right_child_key) {
                        Ordering::Less | Ordering::Equal => right_child,
                        Ordering::Greater => left_child,
                    }
                }
                None => left_child,
            };
            let max_child_key = self.heap_entry(max_child);
            if self.cmp_priorities(pivot_key, max_child_key) == Ordering::Less {
                // Child is greater than element -> move it upwards.
                self.update_position(max_child_key, cursor);
                cursor = max_child;
            } else {
                break 'perculate
            }
        }
        self.update_position(pivot_key, cursor);
    }

    /// Returns a shared reference to the current maximum key and its priority.
    ///
    /// This does not pop the maximum element from the bounded heap.
    pub fn peek(&self) -> Option<(K, W)> {
        if self.is_empty() {
            return None
        }
        let key = self.heap_entry(HeapPosition::root());
        let priority = self.get_priority(key);
        Some((key, priority))
    }

    /// Pops the current maximum key and its priority from the bounded heap.
    pub fn pop(&mut self) -> Option<(K, W)> {
        if self.is_empty() {
            return None
        }
        let key = self.heap_entry(HeapPosition::root());
        self.positions
            .update(key, None)
            .expect("invalid root key of non-empty heap");
        let priority = self.get_priority(key);
        if self.len == 1 {
            // No need to adjust heap properties.
            self.len = 0;
        } else {
            // Replace root with the last element of the heap.
            let new_root = self.heap_entry(HeapPosition::from_index(self.len - 1));
            self.update_position(new_root, HeapPosition::root());
            self.len -= 1;
            self.sift_down(HeapPosition::root());
        }
        Some((key, priority))
    }

    /// Updates the keys heap position.
    ///
    /// # Panics
    ///
    /// - If the given key is out of bounds for the bounded heap.
    /// - If the given heap position is out of bouds for the bounded heap.
    fn update_position(&mut self, key: K, position: HeapPosition) {
        self.heap
            .update(position, key)
            .expect("unexpected out of bounds heap position (heap update)");
        self.positions
            .update(key, Some(position))
            .expect("unexpected out of bounds key (heap update)");
    }

    /// Returns the priority associated with the given key.
    ///
    /// # Panics
    ///
    /// If the key is out of bounds for the bounded heap.
    fn get_priority(&self, key: K) -> W {
        *self
            .priorities
            .get(key)
            .expect("unexpected out of bounds key (get priority)")
    }

    /// Returns the heap position of the given key.
    ///
    /// Returns `None` if the key is currently not contained in the heap.
    ///
    /// # Panics
    ///
    /// If the key if out of bounds for the bounded heap.
    fn get_position(&self, key: K) -> Option<HeapPosition> {
        *self
            .positions
            .get(key)
            .expect("unexpected out of bounds key (get position)")
    }

    /// Returns the heap entry for the given heap position.
    ///
    /// # Panics
    ///
    /// If the given heap position is invalid for the bounded heap.
    fn heap_entry(&self, position: HeapPosition) -> K {
        assert!(position.into_index() < self.len());
        *self
            .heap
            .get(position)
            .expect("encountered out of bounds heap position (query entry)")
    }

    /// Returns `true` if the heap property is satisfied for all elements in the bounded heap.
    ///
    /// # Note
    ///
    /// The heap property is that the priority of parent nodes is always greater than or equal
    /// to the priority of their children.
    ///
    /// This is a test-only API and generally not available.
    fn satisfies_heap_property(&self) -> bool {
        for i in 1..self.len() {
            let child = HeapPosition::from_index(i);
            let parent = child.parent().expect("encountered missing parent");
            let child_key = self.heap_entry(child);
            let parent_key = self.heap_entry(parent);
            if self.cmp_priorities(parent_key, child_key) != Ordering::Greater {
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
        heap.resize_capacity(10);
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.capacity(), 10);
        assert!(heap.is_empty());
    }

    #[test]
    fn empty_heap_contains_no_elements() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.resize_capacity(10);
        for i in 0..size {
            assert_eq!(heap.contains(i), Ok(false));
        }
    }

    #[test]
    fn single_element_heap_contains_exactly_one_element() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.resize_capacity(10);
        heap.push_or_update(5, |_| 42).unwrap();
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
        heap.resize_capacity(size);
        heap.push_or_update(5, |_| 42).unwrap();
        heap.push_or_update(5, |_| 42).unwrap();
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.pop(), Some((5, 42)));
        assert!(heap.is_empty());
    }

    #[test]
    fn single_element_heap_is_empty_after_pop() {
        let size = 10;
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.resize_capacity(size);
        heap.push_or_update(5, |_| 42).unwrap();
        assert_eq!(heap.len(), 1);
        assert_eq!(heap.pop(), Some((5, 42)));
        assert!(heap.is_empty());
    }

    #[test]
    fn satisfies_heap_property_after_insertion() {
        let test_priorities = [3, 9, 1, -5, -10, -9, 10, 0, -1, 7];
        let size = test_priorities.len();
        let mut heap = <BoundedHeap<usize, i32>>::default();
        heap.resize_capacity(size);
        for (k, w) in test_priorities.iter().copied().enumerate() {
            heap.push_or_update(k, |_| w).unwrap();
        }
        assert_eq!(heap.len(), test_priorities.len());
        assert!(heap.satisfies_heap_property());
    }

    #[test]
    fn heap_can_be_filled_to_max() {
        let len = 10;
        let mut heap = BoundedHeap::default();
        heap.resize_capacity(len);
        for (k, w) in (0..len).map(|i| i * 10).enumerate() {
            heap.push_or_update(k, |_| w).unwrap();
        }
        assert_eq!(heap.len(), 10);
    }

    #[test]
    fn out_of_bounds_key_is_rejected() {
        let len = 10;
        let mut heap = BoundedHeap::default();
        heap.resize_capacity(len);
        assert_eq!(
            heap.push_or_update(10, |_| 42),
            Err(OutOfBoundsAccess)
        );
    }

    #[test]
    fn has_descending_removal_sequence() {
        let test_priorities = [3, 9, 1, -5, -10, -9, 10, 0, -1, 7];
        let len = test_priorities.len();
        let mut heap = BoundedHeap::default();
        heap.resize_capacity(len);
        for (k, w) in test_priorities.iter().copied().enumerate() {
            heap.push_or_update(k, |_| w).unwrap();
        }
        assert!(heap.satisfies_heap_property());
        let mut removed_sequence = Vec::new();
        while let Some((k, w)) = heap.pop() {
            removed_sequence.push(w);
            assert!(
                heap.satisfies_heap_property(),
                "heap property NOT satisfied after popping key {}",
                k
            );
        }
        let expected_sequence = {
            let mut priorities = test_priorities.to_vec();
            priorities.sort_by_key(|k| core::cmp::Reverse(*k));
            priorities
        };
        assert_eq!(removed_sequence, expected_sequence);
        assert!(heap.is_empty());
    }

    #[test]
    fn push_pop_sequence_works() {
        let test_weights = [3, 9, 1, -5];
        let len = test_weights.len();
        let mut heap = BoundedHeap::default();
        heap.resize_capacity(10);
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, |_| w).unwrap();
        }
        assert_eq!(heap.pop(), Some((1, 9)));
        assert_eq!(heap.pop(), Some((0, 3)));
        heap.push_or_update(len, |_| 2).unwrap();
        assert_eq!(heap.pop(), Some((len, 2)));
        heap.push_or_update(len + 1, |_| -3).unwrap();
        assert_eq!(heap.pop(), Some((2, 1)));
        assert_eq!(heap.pop(), Some((len + 1, -3)));
        assert_eq!(heap.pop(), Some((3, -5)));
    }

    #[test]
    fn heap_can_be_resized() {
        let test_weights = [10, 30, 20];
        let len = test_weights.len();
        let mut heap = BoundedHeap::default();
        heap.resize_capacity(len);
        for (k, w) in test_weights.iter().copied().enumerate() {
            heap.push_or_update(k, |_| w).unwrap();
        }
        assert_eq!(heap.len(), len);
        assert_eq!(
            heap.push_or_update(len, |_| 40),
            Err(OutOfBoundsAccess)
        );
        heap.resize_capacity(len + 1);
        heap.push_or_update(len, |_| 40).unwrap();
        assert_eq!(heap.len(), len + 1);
        assert_eq!(heap.pop(), Some((len, 40)));
    }
}
