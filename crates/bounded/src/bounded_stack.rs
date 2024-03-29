use super::OutOfBoundsAccess;
use alloc::vec::Vec;
use core::{
    ops::{
        Index,
        IndexMut,
    },
    slice,
    slice::SliceIndex,
};

/// A stack that is bound to a given maximum size.
///
/// # Note
///
/// This is useful to prevent accidental heap memory allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedStack<T> {
    /// Stores the current capacity of the bounded stack.
    ///
    /// # Note
    ///
    /// We cannot re-use the capacity of the underlying vector
    /// since it cannot be relied upon.
    capacity: usize,
    /// The underlying unbounded stack.
    stack: Vec<T>,
}

impl<T> Default for BoundedStack<T> {
    #[inline]
    fn default() -> Self {
        Self {
            capacity: 0,
            stack: Vec::default(),
        }
    }
}

impl<T> BoundedStack<T> {
    /// Resizes the capacity of the bounded stack.
    ///
    /// # Note
    ///
    /// A capacity of N means that the bounded stack may use indices up to N-1
    /// and will bail out errors if used with higher indices.
    #[inline]
    pub fn resize_capacity(&mut self, new_cap: usize) {
        assert!(new_cap >= self.capacity());
        let additional_capacity = new_cap - self.capacity();
        self.capacity += additional_capacity;
        self.stack.reserve_exact(self.capacity() - self.len());
        assert_eq!(self.capacity(), new_cap);
    }

    /// Returns the length of the bounded stack.
    #[inline]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Returns the capacity of the bounded stack.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns `true` if the bounded stack is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the bounded stack is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Returns a shared reference to the last value of the stack if any.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.stack.last()
    }

    /// Returns an exclusive reference to the last value of the stack if any.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    /// Pushes the value to the bounded stack.
    ///
    /// # Errors
    ///
    /// If the bounded stack is full already.
    #[inline]
    pub fn try_push(&mut self, new_value: T) -> Result<(), OutOfBoundsAccess> {
        if self.len() == self.capacity() {
            return Err(OutOfBoundsAccess)
        }
        self.stack.push(new_value);
        Ok(())
    }

    /// Pushes the value to the bounded stack.
    ///
    /// # Panics
    ///
    /// If the bounded stack is full already.
    #[inline]
    pub fn push(&mut self, new_value: T) {
        self.try_push(new_value).unwrap_or_else(|_| {
            panic!(
                "pushed more elements to bounded stack than its capacity of {}",
                self.capacity()
            )
        })
    }

    /// Pops the last value from the bounded stack if any.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Pops the latest values from the bounded stack until it reaches the new length.
    ///
    /// # Panics
    ///
    /// If the new length is larger than the current length.
    #[inline]
    pub fn pop_to<F>(&mut self, new_len: usize, mut observer: F)
    where
        F: FnMut(T),
    {
        assert!(
            new_len <= self.len(),
            "tried to pop a bounded stack with len {} to len {}",
            self.len(),
            new_len,
        );
        for popped in self.stack.drain(new_len..).rev() {
            observer(popped);
        }
        debug_assert_eq!(self.len(), new_len);
    }

    /// Returns an iterator yielding shared references to the values of the bounded stack.
    #[inline]
    pub fn iter(&self) -> slice::Iter<T> {
        self.stack.iter()
    }

    /// Returns an iterator yielding exclusive references to the values of the bounded stack.
    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.stack.iter_mut()
    }

    /// Returns a shared reference to the element at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.stack.get(index)
    }

    /// Returns an exclusive reference to the element at the given index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.stack.get_mut(index)
    }
}

impl<'a, T> IntoIterator for &'a BoundedStack<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut BoundedStack<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, I> Index<I> for BoundedStack<T>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.stack[index]
    }
}

impl<T, I> IndexMut<I> for BoundedStack<T>
where
    I: SliceIndex<[T]>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.stack[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_capacity_works() {
        let mut stack = <BoundedStack<i32>>::default();
        stack.resize_capacity(5);
        assert_eq!(stack.capacity(), 5);
        stack.resize_capacity(10);
        assert_eq!(stack.capacity(), 10);
        stack.push(1);
        assert_eq!(stack.len(), 1);
        stack.resize_capacity(15);
        assert_eq!(stack.capacity(), 15);
    }
}
