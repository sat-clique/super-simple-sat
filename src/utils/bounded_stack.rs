use super::Error;
use core::slice;

/// A stack that is bound to a given maximum size.
#[derive(Debug, PartialEq, Eq)]
pub struct BoundedStack<T> {
    stack: Vec<T>,
}

impl<T> Default for BoundedStack<T> {
    fn default() -> Self {
        Self { stack: Vec::default() }
    }
}

impl<T> BoundedStack<T> {
    /// Increases the capacity of the bounded stack to the new capacity.
    ///
    /// # Errors
    ///
    /// If the new capacity is less than the current capacity.
    pub fn increase_capacity_to(&mut self, new_cap: usize) -> Result<(), Error> {
        if self.capacity() > new_cap {
            return Err(Error::InvalidSizeIncrement)
        }
        self.stack.reserve(new_cap - self.capacity());
        Ok(())
    }

    /// Returns the length of the bounded stack.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Returns `true` if the bounded stack is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the bounded stack is full.
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Returns the capacity of the bounded stack.
    pub fn capacity(&self) -> usize {
        self.stack.capacity()
    }

    /// Returns a shared reference to the last value of the stack if any.
    pub fn last(&self) -> Option<&T> {
        self.stack.last()
    }

    /// Returns an exclusive reference to the last value of the stack if any.
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    /// Pushes the value to the bounded stack.
    ///
    /// # Errors
    ///
    /// If the bounded stack is full already.
    pub fn push(&mut self, new_value: T) -> Result<(), Error> {
        if self.len() == self.capacity() {
            return Err(Error::OutOfBoundsAccess)
        }
        self.stack.push(new_value);
        Ok(())
    }

    /// Pops the last value from the bounded stack if any.
    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Pops the latest values from the bounded stack until it reaches the new length.
    ///
    /// # Errors
    ///
    /// If the new length is greater than the current length.
    pub fn pop_to<F>(&mut self, new_len: usize, mut observer: F) -> Result<(), Error>
    where
        F: FnMut(&T),
    {
        if self.len() < new_len {
            return Err(Error::InvalidSizeDecrement)
        }
        let popped_amount = self.len() - new_len;
        for popped in self.iter().rev().take(popped_amount) {
            observer(popped);
        }
        self.stack.truncate(new_len);
        Ok(())
    }

    /// Returns an iterator yielding shared references to the values of the bounded stack.
    pub fn iter(&self) -> slice::Iter<T> {
        self.stack.iter()
    }

    /// Returns an iterator yielding exclusive references to the values of the bounded stack.
    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.stack.iter_mut()
    }
}

impl<'a, T> IntoIterator for &'a BoundedStack<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut BoundedStack<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}