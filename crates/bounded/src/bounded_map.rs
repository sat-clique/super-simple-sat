use super::{
    BoundedArray,
    Index,
    OutOfBoundsAccess,
};
use core::{
    iter::IntoIterator,
    marker::PhantomData,
    ops,
};

/// A map with a bounded size for index-like keys to value mappings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMap<K, V> {
    /// The current length of the bounded map.
    len: usize,
    /// The underlying values of the bounded map.
    slots: BoundedArray<K, Option<V>>,
    marker: PhantomData<fn() -> K>,
}

impl<K, V> Default for BoundedMap<K, V> {
    fn default() -> Self {
        Self {
            len: 0,
            slots: BoundedArray::default(),
            marker: Default::default(),
        }
    }
}

impl<K, V> BoundedMap<K, V> {
    /// Resizes the capacity of the bounded map.
    ///
    /// # Note
    ///
    /// A capacity of N means that the bounded map may use indices up to N-1
    /// and will bail out errors if used with higher indices.
    #[inline]
    pub fn resize_capacity(&mut self, new_len: usize) {
        self.slots.resize_with(new_len, Default::default);
    }

    /// Returns the current length of the bounded map.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the bounded map is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the bounded map is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Returns the total capacity of the bounded map.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

impl<K, V> BoundedMap<K, V>
where
    K: Index,
{
    /// Creates a new bounded map with the given capacity.
    ///
    /// # Note
    ///
    /// A capacity of N means that the bounded map may store up to N different
    /// mappings and will error otherwise.
    pub fn with_capacity(len: usize) -> Self {
        Self {
            len: 0,
            slots: BoundedArray::with_len(len, |_| Default::default()),
            marker: Default::default(),
        }
    }

    /// Inserts the given value for the key and returns the old value if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    #[inline]
    pub fn insert(
        &mut self,
        index: K,
        new_value: V,
    ) -> Result<Option<V>, OutOfBoundsAccess> {
        let old_value = self.slots.get_mut(index)?.replace(new_value);
        if old_value.is_none() {
            self.len += 1;
        }
        Ok(old_value)
    }

    /// Takes the value of the given key and returns it if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    #[inline]
    pub fn take(&mut self, index: K) -> Result<Option<V>, OutOfBoundsAccess> {
        let old_value = self.slots.get_mut(index)?.take();
        if old_value.is_some() {
            self.len -= 1;
        }
        Ok(old_value)
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
    fn get_impl(&self, index: K) -> Result<&Option<V>, OutOfBoundsAccess> {
        self.slots.get(index)
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
    fn get_mut_impl(&mut self, index: K) -> Result<&mut Option<V>, OutOfBoundsAccess> {
        self.slots.get_mut(index)
    }

    /// Returns a shared reference to the value for the given key if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    #[inline]
    pub fn get(&self, index: K) -> Result<Option<&V>, OutOfBoundsAccess> {
        self.get_impl(index).map(Into::into)
    }

    /// Returns an exclusive reference to the value for the given key if any.
    ///
    /// # Error
    ///
    /// Returns an error if the key's index is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: K) -> Result<Option<&mut V>, OutOfBoundsAccess> {
        self.get_mut_impl(index).map(Into::into)
    }

    /// Returns an iterator yielding shared references to the key and value pairs.
    #[inline]
    pub fn iter(&self) -> Iter<K, V> {
        Iter::new(self)
    }

    /// Returns an iterator yielding exclusive references to the key and value pairs.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut::new(self)
    }
}

impl<'a, K, V> IntoIterator for &'a BoundedMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut BoundedMap<K, V>
where
    K: Index,
{
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
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

    #[inline]
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

    #[inline]
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
    #[inline]
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
    #[inline]
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
        assert_eq!(map.get(3), Err(OutOfBoundsAccess));
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
        assert_eq!(map.insert(3, b'D'), Err(OutOfBoundsAccess));
        assert_eq!(map.get(3), Err(OutOfBoundsAccess));
        assert_eq!(map.get_mut(3), Err(OutOfBoundsAccess));
    }

    #[test]
    fn increase_size_works() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(0);
        assert!(map.is_empty());
        assert!(map.is_full());
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 0);
        assert_eq!(map.into_iter().next(), None);
        assert_eq!(map.get(0), Err(OutOfBoundsAccess));
        // Now increase size to 3:
        map.resize_capacity(3);
        assert!(map.is_empty());
        assert!(!map.is_full());
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 3);
        assert_eq!(map.into_iter().next(), None);
        assert_eq!(map.get(0), Ok(None));
        // Increase to same size works, too.
        map.resize_capacity(3);
    }

    #[test]
    fn shrink_size_works() {
        let mut map = <BoundedMap<usize, u8>>::with_capacity(3);
        map.resize_capacity(2);
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 2);
    }
}
