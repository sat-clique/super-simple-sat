use super::{
    Error,
    Index,
};
use core::{
    iter::FromIterator,
    marker::PhantomData,
    ops,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedArray<Idx, T> {
    values: Vec<T>,
    marker: PhantomData<fn() -> Idx>,
}

impl<Idx, T> Default for BoundedArray<Idx, T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            values: Vec::new(),
            marker: Default::default(),
        }
    }
}

impl<Idx, T> FromIterator<T> for BoundedArray<Idx, T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            values: iter.into_iter().collect(),
            marker: Default::default(),
        }
    }
}

impl<Idx, T> BoundedArray<Idx, T> {
    /// Returns the current length of the bounded array.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl<Idx, T> BoundedArray<Idx, T>
where
    Idx: Index,
{
    /// Ensures that the given index is valid for the bounded array.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds.
    fn ensure_valid_index(&self, index: Idx) -> Result<usize, Error> {
        let index = index.into_index();
        if index >= self.len() {
            return Err(Error::OutOfBoundsAccess)
        }
        Ok(index)
    }

    /// Returns a shared reference to the element at the given index.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds for the bounded array.
    pub fn get(&self, index: Idx) -> Result<&T, Error> {
        self.ensure_valid_index(index)
            .map(move |index| &self.values[index])
    }

    /// Returns an exclusive reference to the element at the given index.
    ///
    /// # Errors
    ///
    /// If the given index is out of bounds for the bounded array.
    pub fn get_mut(&mut self, index: Idx) -> Result<&mut T, Error> {
        self.ensure_valid_index(index)
            .map(move |index| &mut self.values[index])
    }

    /// Swaps the elements at the given indices.
    ///
    /// # Errors
    ///
    /// If any of the given indices is out of bounds for the bounded array.
    pub fn swap(&mut self, lhs: Idx, rhs: Idx) -> Result<(), Error> {
        let lhs = self.ensure_valid_index(lhs)?;
        let rhs = self.ensure_valid_index(rhs)?;
        self.values.swap(lhs, rhs);
        Ok(())
    }
}

impl<Idx, T> BoundedArray<Idx, T> {
    /// Returns an iterator yielding shared references over the array values.
    pub fn iter(&self) -> core::slice::Iter<T> {
        self.values.iter()
    }

    /// Returns an iterator yielding exclusive references over the array values.
    pub fn iter_mut(&mut self) -> core::slice::IterMut<T> {
        self.values.iter_mut()
    }
}

impl<Idx, T> BoundedArray<Idx, T>
where
    T: Default,
{
    /// Creates a new bounded array with the given length.
    ///
    /// Initializes all slots of the array with default values.
    pub fn with_len(len: usize) -> Self {
        Self {
            values: (0..len).map(|_| Default::default()).collect(),
            marker: Default::default(),
        }
    }

    /// Increases the length of the bounded array to the given new length.
    ///
    /// Fills all additional slots with default values.
    pub fn increase_len_to(&mut self, new_len: usize) -> Result<(), Error> {
        if self.len() > new_len {
            return Err(Error::InvalidSizeIncrement)
        }
        self.values.resize_with(new_len, || Default::default());
        Ok(())
    }
}

impl<'a, Idx, T> IntoIterator for &'a BoundedArray<Idx, T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, Idx, T> IntoIterator for &'a mut BoundedArray<Idx, T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<Idx, T> ops::Index<Idx> for BoundedArray<Idx, T>
where
    Idx: Index,
{
    type Output = T;

    /// Returns a shared reference to the value for the given index if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the index is out of bounds.
    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index).expect("encountered out of bounds index")
    }
}

impl<Idx, T> ops::IndexMut<Idx> for BoundedArray<Idx, T>
where
    Idx: Index,
{
    /// Returns an exclusive reference to the value for the given index if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the index is out of bounds.
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index)
            .expect("encountered out of bounds index")
    }
}
