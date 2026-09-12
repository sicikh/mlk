use std::{iter::Enumerate, marker::PhantomData};

use crate::{ArenaIndex, ArenaToken, RawIdx};

/// A map from arena indexes to some other type.
/// Space requirement is O(highest index).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArenaMap<IDX, V> {
    v: Vec<Option<V>>,
    _ty: PhantomData<IDX>,
}

impl<IDX: ArenaIndex, V> ArenaMap<IDX, V> {
    /// Creates a new empty map.
    pub const fn new() -> Self {
        Self {
            v: Vec::new(),
            _ty: PhantomData,
        }
    }

    /// Create a new empty map with specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            v: Vec::with_capacity(capacity),
            _ty: PhantomData,
        }
    }

    /// Reserves capacity for at least additional more elements to be inserted in the map.
    pub fn reserve(&mut self, additional: usize) {
        self.v.reserve(additional);
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self) {
        self.v.clear();
    }

    /// Shrinks the capacity of the map as much as possible.
    pub fn shrink_to_fit(&mut self) {
        let min_len = self
            .v
            .iter()
            .rposition(|slot| slot.is_some())
            .map_or(0, |i| i + 1);
        self.v.truncate(min_len);
        self.v.shrink_to_fit();
    }

    /// Returns whether the map contains a value for the specified index.
    pub fn contains_idx(&self, idx: IDX) -> bool {
        matches!(self.v.get(Self::idx_to_index(idx)), Some(Some(_)))
    }

    /// Removes an index from the map, returning the value at the index if the index was previously in the map.
    pub fn remove(&mut self, idx: IDX) -> Option<V> {
        self.v.get_mut(Self::idx_to_index(idx))?.take()
    }

    /// Inserts a value associated with a given arena index into the map.
    ///
    /// If the map did not have this index present, None is returned.
    /// Otherwise, the value is updated, and the old value is returned.
    pub fn insert(&mut self, idx: IDX, t: V) -> Option<V> {
        let idx = Self::idx_to_index(idx);

        self.v.resize_with((idx + 1).max(self.v.len()), || None);
        self.v[idx].replace(t)
    }

    /// Returns a reference to the value associated with the provided index
    /// if it is present.
    pub fn get(&self, idx: IDX) -> Option<&V> {
        self.v
            .get(Self::idx_to_index(idx))
            .and_then(|it| it.as_ref())
    }

    /// Returns a mutable reference to the value associated with the provided index
    /// if it is present.
    pub fn get_mut(&mut self, idx: IDX) -> Option<&mut V> {
        self.v
            .get_mut(Self::idx_to_index(idx))
            .and_then(|it| it.as_mut())
    }

    /// Returns an iterator over the values in the map.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> {
        self.v.iter().filter_map(|o| o.as_ref())
    }

    /// Returns an iterator over mutable references to the values in the map.
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut V> {
        self.v.iter_mut().filter_map(|o| o.as_mut())
    }

    /// Returns an iterator over the arena indexes and values in the map.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (IDX, &V)> {
        self.v
            .iter()
            .enumerate()
            .filter_map(|(idx, o)| Some((Self::idx_from_index(idx), o.as_ref()?)))
    }

    /// Returns an iterator over the arena indexes and values in the map.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IDX, &mut V)> {
        self.v
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, o)| Some((Self::idx_from_index(idx), o.as_mut()?)))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, idx: IDX) -> Entry<'_, IDX, V> {
        let idx = Self::idx_to_index(idx);
        self.v.resize_with((idx + 1).max(self.v.len()), || None);
        match &mut self.v[idx] {
            slot @ Some(_) => {
                Entry::Occupied(OccupiedEntry {
                    slot,
                    _ty: PhantomData,
                })
            },
            slot => {
                Entry::Vacant(VacantEntry {
                    slot,
                    _ty: PhantomData,
                })
            },
        }
    }

    fn idx_to_index(idx: IDX) -> usize {
        idx.into_raw().to_index() as usize
    }

    fn idx_from_index(index: usize) -> IDX {
        IDX::from_raw(RawIdx::from_index(index as u32), ArenaToken(()))
    }
}

impl<IDX: ArenaIndex, V> std::ops::Index<IDX> for ArenaMap<IDX, V> {
    type Output = V;
    fn index(&self, idx: IDX) -> &V {
        self.v[Self::idx_to_index(idx)].as_ref().unwrap()
    }
}

impl<IDX: ArenaIndex, V> std::ops::IndexMut<IDX> for ArenaMap<IDX, V> {
    fn index_mut(&mut self, idx: IDX) -> &mut V {
        self.v[Self::idx_to_index(idx)].as_mut().unwrap()
    }
}

impl<IDX: ArenaIndex, V> Default for ArenaMap<IDX, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<IDX: ArenaIndex, V> Extend<(IDX, V)> for ArenaMap<IDX, V> {
    fn extend<I: IntoIterator<Item = (IDX, V)>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |(k, v)| {
            self.insert(k, v);
        });
    }
}

impl<IDX: ArenaIndex, V> FromIterator<(IDX, V)> for ArenaMap<IDX, V> {
    fn from_iter<I: IntoIterator<Item = (IDX, V)>>(iter: I) -> Self {
        let mut this = Self::new();
        this.extend(iter);
        this
    }
}

pub struct ArenaMapIter<IDX, V> {
    iter: Enumerate<std::vec::IntoIter<Option<V>>>,
    _ty: PhantomData<IDX>,
}

impl<IDX: ArenaIndex, V> IntoIterator for ArenaMap<IDX, V> {
    type Item = (IDX, V);

    type IntoIter = ArenaMapIter<IDX, V>;

    fn into_iter(self) -> Self::IntoIter {
        let iter = self.v.into_iter().enumerate();
        Self::IntoIter {
            iter,
            _ty: PhantomData,
        }
    }
}

impl<IDX: ArenaIndex, V> ArenaMapIter<IDX, V> {
    fn mapper((idx, o): (usize, Option<V>)) -> Option<(IDX, V)> {
        Some((ArenaMap::<IDX, V>::idx_from_index(idx), o?))
    }
}

impl<IDX: ArenaIndex, V> Iterator for ArenaMapIter<IDX, V> {
    type Item = (IDX, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.by_ref().find_map(Self::mapper)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<IDX: ArenaIndex, V> DoubleEndedIterator for ArenaMapIter<IDX, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.by_ref().rev().find_map(Self::mapper)
    }
}

/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This `enum` is constructed from the [`entry`] method on [`ArenaMap`].
///
/// [`entry`]: ArenaMap::entry
pub enum Entry<'a, IDX, V> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, IDX, V>),
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, IDX, V>),
}

impl<'a, IDX, V> Entry<'a, IDX, V> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable reference to
    /// the value in the entry.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Self::Vacant(ent) => ent.insert(default),
            Self::Occupied(ent) => ent.into_mut(),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty, and returns
    /// a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Self::Vacant(ent) => ent.insert(default()),
            Self::Occupied(ent) => ent.into_mut(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Self::Occupied(ent) = &mut self {
            f(ent.get_mut());
        }
        self
    }
}

impl<'a, IDX, V> Entry<'a, IDX, V>
where
    V: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty, and returns a mutable reference
    /// to the value in the entry.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(Default::default)
    }
}

/// A view into an vacant entry in a [`ArenaMap`]. It is part of the [`Entry`] enum.
pub struct VacantEntry<'a, IDX, V> {
    slot: &'a mut Option<V>,
    _ty: PhantomData<IDX>,
}

impl<'a, IDX, V> VacantEntry<'a, IDX, V> {
    /// Sets the value of the entry with the `VacantEntry`’s key, and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V {
        self.slot.insert(value)
    }
}

/// A view into an occupied entry in a [`ArenaMap`]. It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a, IDX, V> {
    slot: &'a mut Option<V>,
    _ty: PhantomData<IDX>,
}

impl<'a, IDX, V> OccupiedEntry<'a, IDX, V> {
    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        self.slot.as_ref().expect("Occupied")
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        self.slot.as_mut().expect("Occupied")
    }

    /// Converts the entry into a mutable reference to its value.
    pub fn into_mut(self) -> &'a mut V {
        self.slot.as_mut().expect("Occupied")
    }

    /// Sets the value of the entry with the `OccupiedEntry`’s key, and returns the entry’s old value.
    pub fn insert(&mut self, value: V) -> V {
        self.slot.replace(value).expect("Occupied")
    }

    /// Takes the value of the entry out of the map, and returns it.
    pub fn remove(self) -> V {
        self.slot.take().expect("Occupied")
    }
}
