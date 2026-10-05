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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arena, Idx};

    fn ids(count: usize) -> Vec<Idx<()>> {
        let mut arena = Arena::new();
        (0..count).map(|_| arena.alloc(())).collect()
    }

    #[test]
    fn inserting_a_new_index_returns_none_and_replacing_returns_the_old_value() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();

        assert_eq!(map.insert(a, 1), None);
        assert!(map.contains_idx(a));
        assert_eq!(map.get(a), Some(&1));

        assert_eq!(map.insert(a, 2), Some(1));
        assert_eq!(map.get(a), Some(&2));
    }

    #[test]
    fn a_gap_between_indexes_stays_vacant() {
        let ids = ids(4);
        let (a, b, c, d) = (ids[0], ids[1], ids[2], ids[3]);

        let mut map = ArenaMap::new();
        map.insert(a, 1);
        map.insert(c, 3);

        assert!(!map.contains_idx(b));
        assert_eq!(map.get(b), None);
        assert_eq!(map.remove(b), None);

        // The gap lies inside the map, and the index at its end is absent too.
        assert!(!map.contains_idx(d));
        assert_eq!(map.get(d), None);
        assert_eq!(map.iter().collect::<Vec<_>>(), [(a, &1), (c, &3)]);
    }

    #[test]
    fn removing_an_entry_vacates_its_index_for_reuse() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();
        map.insert(a, 1);

        assert_eq!(map.remove(a), Some(1));
        assert!(!map.contains_idx(a));
        assert_eq!(map.get(a), None);
        assert_eq!(map.remove(a), None);

        map.insert(a, 2);
        assert_eq!(map.get(a), Some(&2));
        assert_eq!(map.iter().collect::<Vec<_>>(), [(a, &2)]);
    }

    #[test]
    fn entries_iterate_in_index_order_and_skip_vacancies() {
        let ids = ids(3);
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        let mut map = ArenaMap::new();
        map.insert(c, 3);
        map.insert(a, 1);
        map.insert(b, 2);
        map.remove(b);

        assert_eq!(map.values().collect::<Vec<_>>(), [&1, &3]);
        assert_eq!(map.values().rev().collect::<Vec<_>>(), [&3, &1]);
        assert_eq!(map.iter().collect::<Vec<_>>(), [(a, &1), (c, &3)]);
        assert_eq!(map.iter().rev().collect::<Vec<_>>(), [(c, &3), (a, &1)]);
    }

    #[test]
    fn values_mut_and_iter_mut_write_through() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let mut map = ArenaMap::new();
        map.insert(a, 1);
        map.insert(b, 2);

        for value in map.values_mut() {
            *value *= 10;
        }
        for (_, value) in map.iter_mut() {
            *value += 1;
        }

        assert_eq!(map.get(a), Some(&11));
        assert_eq!(map.get(b), Some(&21));
    }

    #[test]
    fn into_iter_yields_entries_in_index_order_and_is_double_ended() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let mut map = ArenaMap::new();
        map.insert(b, 2);
        map.insert(a, 1);

        assert_eq!(map.clone().into_iter().collect::<Vec<_>>(), [
            (a, 1),
            (b, 2)
        ]);
        assert_eq!(map.into_iter().rev().collect::<Vec<_>>(), [(b, 2), (a, 1)]);
    }

    #[test]
    fn entry_reports_whether_an_index_is_vacant_or_occupied() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();

        assert!(matches!(map.entry(a), Entry::Vacant(_)));

        map.insert(a, 1);

        assert!(matches!(map.entry(a), Entry::Occupied(_)));
    }

    #[test]
    fn or_insert_keeps_an_existing_value() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();

        assert_eq!(*map.entry(a).or_insert(1), 1);
        assert_eq!(*map.entry(a).or_insert(2), 1);
        assert_eq!(map.get(a), Some(&1));
    }

    #[test]
    fn or_insert_with_builds_the_default_only_when_vacant() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();
        let mut calls = 0;

        let first = *map.entry(a).or_insert_with(|| {
            calls += 1;
            1
        });
        let second = *map.entry(a).or_insert_with(|| {
            calls += 1;
            2
        });

        assert_eq!(first, 1);
        assert_eq!(second, 1);
        assert_eq!(calls, 1);
    }

    #[test]
    fn or_default_fills_a_vacant_entry() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();

        assert_eq!(*map.entry(a).or_default(), 0);

        *map.entry(a).or_default() = 7;

        assert_eq!(map.get(a), Some(&7));
    }

    #[test]
    fn and_modify_applies_to_an_occupied_entry_only() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let mut map = ArenaMap::new();
        map.insert(a, 1);

        map.entry(a).and_modify(|value| *value += 1).or_insert(10);
        map.entry(b).and_modify(|value| *value += 1).or_insert(20);

        assert_eq!(map.get(a), Some(&2));
        assert_eq!(map.get(b), Some(&20));
    }

    #[test]
    fn an_occupied_entry_reads_replaces_and_takes_the_value() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();
        map.insert(a, 1);

        let Entry::Occupied(mut entry) = map.entry(a) else {
            panic!("an inserted index is occupied");
        };

        assert_eq!(*entry.get(), 1);
        *entry.get_mut() += 1;
        assert_eq!(entry.insert(3), 2);
        assert_eq!(*entry.into_mut(), 3);

        assert_eq!(map.get(a), Some(&3));
    }

    #[test]
    fn removing_through_an_occupied_entry_vacates_the_index() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();
        map.insert(a, 1);

        let Entry::Occupied(entry) = map.entry(a) else {
            panic!("an inserted index is occupied");
        };

        assert_eq!(entry.remove(), 1);

        assert!(!map.contains_idx(a));
        assert_eq!(map.get(a), None);
    }

    #[test]
    fn indexing_reaches_a_present_value_and_writes_through() {
        let ids = ids(1);
        let a = ids[0];
        let mut map = ArenaMap::new();
        map.insert(a, 1);

        assert_eq!(map[a], 1);

        map[a] = 5;

        assert_eq!(map[a], 5);
    }

    #[test]
    #[should_panic]
    fn indexing_a_vacant_index_panics() {
        let ids = ids(1);

        let map: ArenaMap<Idx<()>, i32> = ArenaMap::new();
        let _ = map[ids[0]];
    }

    #[test]
    fn clearing_removes_every_entry() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let mut map = ArenaMap::new();
        map.insert(a, 1);
        map.insert(b, 2);

        map.clear();

        assert!(!map.contains_idx(a));
        assert_eq!(map.get(b), None);
        assert_eq!(map.iter().count(), 0);

        map.insert(b, 3);
        assert_eq!(map.get(b), Some(&3));
    }

    #[test]
    fn a_map_is_built_from_pairs_and_extended_with_more() {
        let ids = ids(3);
        let (a, b, c) = (ids[0], ids[1], ids[2]);

        let mut map: ArenaMap<Idx<()>, i32> = [(b, 2), (a, 1)].into_iter().collect();
        map.extend([(c, 3), (a, 10)]);

        assert_eq!(map.get(a), Some(&10));
        assert_eq!(map.get(b), Some(&2));
        assert_eq!(map.get(c), Some(&3));
        assert_eq!(map.iter().map(|(_, value)| *value).collect::<Vec<_>>(), [
            10, 2, 3
        ]);
    }

    #[test]
    fn a_fresh_map_has_no_entry() {
        let ids = ids(1);
        let a = ids[0];

        let empty: ArenaMap<Idx<()>, i32> = ArenaMap::new();
        let defaulted: ArenaMap<Idx<()>, i32> = ArenaMap::default();
        let mut reserved = ArenaMap::with_capacity(8);
        reserved.reserve(8);
        reserved.insert(a, 1);

        assert_eq!(empty.get(a), None);
        assert_eq!(defaulted.get(a), None);
        assert_eq!(reserved.get(a), Some(&1));
    }

    #[test]
    fn shrinking_to_fit_keeps_the_entries_and_drops_the_vacant_tail() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let mut map = ArenaMap::new();
        map.insert(a, 1);
        map.insert(b, 2);
        map.remove(b);

        map.shrink_to_fit();

        assert_eq!(map.get(a), Some(&1));
        assert!(!map.contains_idx(b));
        assert_eq!(map.iter().collect::<Vec<_>>(), [(a, &1)]);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct NodeId(RawIdx);

    impl ArenaIndex for NodeId {
        fn into_raw(self) -> RawIdx {
            self.0
        }

        fn from_raw(raw: RawIdx, _token: ArenaToken) -> Self {
            NodeId(raw)
        }
    }

    #[test]
    fn any_arena_index_type_can_key_a_map() {
        let mut arena = Arena::new();
        let first = NodeId(arena.alloc("a").into_raw());
        let second = NodeId(arena.alloc("b").into_raw());

        let mut map = ArenaMap::new();
        map.insert(second, 2);
        map.insert(first, 1);

        assert_eq!(map.get(first), Some(&1));
        assert_eq!(map.iter().collect::<Vec<_>>(), [(first, &1), (second, &2)]);
    }
}
