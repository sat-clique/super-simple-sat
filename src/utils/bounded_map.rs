use super::{
    Index,
};
use core::{
    iter::{
        FromIterator,
        IntoIterator,
    },
    marker::PhantomData,
    ops,
};

/// A map with a bounded size for index-like keys to value mappings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMap<K, V> {
    /// The current length of the bounded map.
    len: usize,
    /// The underlying values of the bounded map.
    slots: Vec<Option<V>>,
    marker: PhantomData<fn() -> K>,
}

/// Errors that may occure when operating on a bounded map.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    OutOfBoundsAccess,
    InvalidSizeIncrement,
}

impl<K, V> BoundedMap<K, V> {
    /// Creates a new bounded map with the given capacity.
    ///
    /// # Note
    ///
    /// A capacity of N means that the bounded map may store up to N different
    /// mappings and will error otherwise.
    pub fn with_capacity(len: usize) -> Self {
        Self {
            len: 0,
            slots: Vec::from_iter((0..len).map(|_| None)),
            marker: Default::default(),
        }
    }

    /// Increases the capacaity of the bounded map to the new value.
    ///
    /// # Note
    ///
    /// A capacity of N means that the bounded map may store up to N different
    /// mappings and will error otherwise.
    pub fn increase_capacity_to(&mut self, new_len: usize) -> Result<(), Error> {
        if new_len < self.capacity() {
            return Err(Error::InvalidSizeIncrement)
        }
        self.slots.resize_with(new_len, || None);
        Ok(())
    }

    /// Returns the current length of the bounded map.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the bounded map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the bounded map is full.
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Returns the total capacity of the bounded map.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

impl<K, V> BoundedMap<K, V>
where
    K: Index,
{
    /// Ensures that the given index is valid for the capacity of the bounded map.
    ///
    /// Returns the raw value of the index if it is valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
    fn ensure_valid_index(&self, index: K) -> Result<usize, Error> {
        let index = index.into_index();
        if index >= self.capacity() {
            return Err(Error::OutOfBoundsAccess)
        }
        Ok(index)
    }

    /// Inserts the given value for the key and returns the old value if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    pub fn insert(&mut self, index: K, new_value: V) -> Result<Option<V>, Error> {
        self.ensure_valid_index(index).map(|raw_index| {
            let old_value = self.slots[raw_index].replace(new_value);
            if old_value.is_none() {
                self.len += 1;
            }
            old_value
        })
    }

    /// Takes the value of the given key and returns it if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    pub fn take(&mut self, index: K) -> Result<Option<V>, Error> {
        self.ensure_valid_index(index).map(|raw_index| {
            let old_value = self.slots[raw_index].take();
            if old_value.is_some() {
                self.len -= 1;
            }
            old_value
        })
    }

    /// Returns a shared reference to the value for the given key if any.
    ///
    /// # Dev. Note
    ///
    /// This is an internal method that is used by several public methods in
    /// order to provide a shared way of accessing shared values.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    fn get_impl(&self, index: K) -> Result<&Option<V>, Error> {
        self.ensure_valid_index(index)
            .map(move |raw_index| &self.slots[raw_index])
    }

    /// Returns an exclusive reference to the value for the given key if any.
    ///
    /// # Dev. Note
    ///
    /// This is an internal method that is used by several public methods in
    /// order to provide a shared way of accessing exclusive values.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    fn get_mut_impl(&mut self, index: K) -> Result<&mut Option<V>, Error> {
        self.ensure_valid_index(index)
            .map(move |raw_index| &mut self.slots[raw_index])
    }

    /// Returns a shared reference to the value for the given key if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    pub fn get(&self, index: K) -> Result<Option<&V>, Error> {
        self.get_impl(index).map(Into::into)
    }

    /// Returns an exclusive reference to the value for the given key if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    pub fn get_mut(&mut self, index: K) -> Result<Option<&mut V>, Error> {
        self.get_mut_impl(index).map(Into::into)
    }
}

impl<'a, K, V> IntoIterator for &'a BoundedMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a, K, V> IntoIterator for &'a mut BoundedMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut::new(self)
    }
}

pub struct Iter<'a, K, V> {
    iter: core::iter::Enumerate<core::slice::Iter<'a, Option<V>>>,
    marker: PhantomData<fn() -> K>,
}

impl<'a, K, V> Iter<'a, K, V> {
    fn new(bounded_map: &'a BoundedMap<K, V>) -> Self {
        Self {
            iter: bounded_map.slots.iter().enumerate(),
            marker: Default::default(),
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        'find: loop {
            match self.iter.next() {
                Some((index, Some(value))) => return Some((K::from_index(index), value)),
                Some((_, None)) => continue 'find,
                None => return None,
            }
        }
    }
}

pub struct IterMut<'a, K, V> {
    iter: core::iter::Enumerate<core::slice::IterMut<'a, Option<V>>>,
    marker: PhantomData<fn() -> K>,
}

impl<'a, K, V> IterMut<'a, K, V> {
    fn new(bounded_map: &'a mut BoundedMap<K, V>) -> Self {
        Self {
            iter: bounded_map.slots.iter_mut().enumerate(),
            marker: Default::default(),
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        'find: loop {
            match self.iter.next() {
                Some((index, Some(value))) => return Some((K::from_index(index), value)),
                Some((_, None)) => continue 'find,
                None => return None,
            }
        }
    }
}

impl<K, V> ops::Index<K> for BoundedMap<K, V>
where
    K: Index,
{
    type Output = Option<V>;

    /// Returns a shared reference to the value for the given key if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the key's index is out of bounds.
    fn index(&self, index: K) -> &Self::Output {
        self.get_impl(index)
            .expect("encountered out of bounds index")
    }
}

impl<K, V> ops::IndexMut<K> for BoundedMap<K, V>
where
    K: Index,
{
    /// Returns an exclusive reference to the value for the given key if any.
    ///
    /// # Panics
    ///
    /// Returns an error if the key's index is out of bounds.
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut_impl(index)
            .expect("encountered out of bounds index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_capacity_works() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(3);
        assert!(map.is_empty());
        assert!(!map.is_full());
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 3);
        assert_eq!(map.into_iter().next(), None);
        for i in 0..3 {
            assert_eq!(map.get(i), Ok(None));
            assert_eq!(map.get_mut(i), Ok(None));
            assert_eq!((&map)[i], None);
            assert_eq!((&mut map)[i], None);
        }
        assert_eq!(map.get(3), Err(Error::OutOfBoundsAccess));
    }

    #[test]
    fn insert_works() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(3);
        let mut test_values = [b'A', b'B', b'C'];
        assert_eq!(map.len(), 0);
        assert_eq!(map.insert(0, b'A').unwrap(), None);
        assert_eq!(map.insert(1, b'B').unwrap(), None);
        assert_eq!(map.insert(2, b'C').unwrap(), None);
        assert_eq!(map.len(), 3);
        assert!(map.is_full());
        for i in 0..3 {
            assert_eq!(map.get(i), Ok(Some(&test_values[i])));
            assert_eq!(map.get_mut(i), Ok(Some(&mut test_values[i])));
            assert_eq!(map[i], Some(test_values[i]));
        }
        assert_eq!(map.insert(3, b'D'), Err(Error::OutOfBoundsAccess));
        assert_eq!(map.get(3), Err(Error::OutOfBoundsAccess));
        assert_eq!(map.get_mut(3), Err(Error::OutOfBoundsAccess));
    }

    #[test]
    fn increase_size_works() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(0);
        assert!(map.is_empty());
        assert!(map.is_full());
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 0);
        assert_eq!(map.into_iter().next(), None);
        assert_eq!(map.get(0), Err(Error::OutOfBoundsAccess));
        // Now increase size to 3:
        assert!(map.increase_capacity_to(3).is_ok());
        assert!(map.is_empty());
        assert!(!map.is_full());
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 3);
        assert_eq!(map.into_iter().next(), None);
        assert_eq!(map.get(0), Ok(None));
        // Increase to same size works, too.
        assert!(map.increase_capacity_to(3).is_ok());
    }

    #[test]
    fn shrink_size_fails() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(3);
        assert_eq!(map.increase_capacity_to(2), Err(Error::InvalidSizeIncrement));
    }
}
