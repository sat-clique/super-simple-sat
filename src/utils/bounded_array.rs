use super::{Error, Index};
use core::ops;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedArray<T> {
    values: Vec<T>,
}

impl<T> Default for BoundedArray<T>
where
    T: Default,
{
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

impl<T> BoundedArray<T> {
    /// Returns the current length of the bounded array.
    pub fn len(&self) -> usize {
        self.values.capacity()
    }

    /// Ensures that the given index is valid for the bounded array.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds.
    fn ensure_valid_index(&self, index: usize) -> Result<usize, Error> {
        if index >= self.len() {
            return Err(Error::OutOfBoundsAccess)
        }
        Ok(index)
    }

    /// Returns a shared reference to the element at the given index.
    pub fn get(&self, index: usize) -> Result<&T, Error> {
        self.ensure_valid_index(index)
            .map(move |index| &self.values[index])
    }

    /// Returns an exclusive reference to the element at the given index.
    pub fn get_mut(&mut self, index: usize) -> Result<&mut T, Error> {
        self.ensure_valid_index(index)
            .map(move |index| &mut self.values[index])
    }

    /// Returns an iterator yielding shared references over the array values.
    pub fn iter(&self) -> core::slice::Iter<T> {
        self.values.iter()
    }

    /// Returns an iterator yielding exclusive references over the array values.
    pub fn iter_mut(&mut self) -> core::slice::IterMut<T> {
        self.values.iter_mut()
    }
}

impl<T> BoundedArray<T>
where
    T: Default,
{
    /// Creates a new bounded array with the given length.
    ///
    /// Initializes all slots of the array with default values.
    pub fn with_len(len: usize) -> Self {
        Self {
            values: (0..len).map(|_| Default::default()).collect(),
        }
    }

    /// Increases the length of the bounded array to the given new length.
    ///
    /// Fills all additional slots with default values.
    pub fn increase_to_capacity(&mut self, new_len: usize) -> Result<(), Error> {
        if self.len() > new_len {
            return Err(Error::InvalidSizeIncrement)
        }
        self.values.resize_with(new_len, || Default::default());
        Ok(())
    }
}

impl<'a, T> IntoIterator for &'a BoundedArray<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut BoundedArray<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> ops::Index<usize> for BoundedArray<T> {
    type Output = T;

    /// Returns a shared reference to the value for the given index if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the index is out of bounds.
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("encountered out of bounds index")
    }
}

impl<T> ops::IndexMut<usize> for BoundedArray<T> {
    /// Returns an exclusive reference to the value for the given index if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the index is out of bounds.
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .expect("encountered out of bounds index")
    }
}
