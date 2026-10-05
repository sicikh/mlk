//! Yet another index-based arena with Option-optimized indices.

use std::{
    cmp, fmt,
    hash::{Hash, Hasher},
    iter::{Enumerate, FusedIterator},
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Index, IndexMut},
    range::{Range, RangeInclusive},
};

mod map;
pub use map::{ArenaMap, Entry, OccupiedEntry, VacantEntry};

/// ZST-token to prevent creating invalid [`Idx`]s from [`RawIdx`]s.
#[derive(Debug, Clone, Copy)]
pub struct ArenaToken(());

/// Trait for newtypes around [`RawIdx`].
pub trait ArenaIndex: Copy {
    fn into_raw(self) -> RawIdx;
    fn from_raw(index: RawIdx, token: ArenaToken) -> Self;
}

impl<T> ArenaIndex for Idx<T> {
    fn into_raw(self) -> RawIdx {
        self.into_raw()
    }

    fn from_raw(index: RawIdx, _token: ArenaToken) -> Self {
        Self::from_raw(index)
    }
}

/// The [`Option`]-optimized raw index of a value in an [`Arena`].
///
/// Value of 0 is reserved for [`None`] by disallowing value of [`u32::MAX`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawIdx(NonZeroU32);

const _: () = assert!(size_of::<Option<Idx<()>>>() == size_of::<Idx<()>>());

impl RawIdx {
    /// Constructs a [`RawIdx`] from a [`NonZeroU32`].
    ///
    /// ## Panics
    ///
    /// Panics if provided value is [`u32::MAX`].
    #[inline]
    pub const fn new(u32: NonZeroU32) -> Self {
        assert!(u32.get() != u32::MAX);
        RawIdx(u32)
    }

    /// Constructs a [`RawIdx`] from a [`NonZeroU32`] without checking the value.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the provided [`NonZeroU32`] is less than [`u32::MAX`].
    #[inline]
    pub const unsafe fn new_unchecked(u32: NonZeroU32) -> Self {
        RawIdx(u32)
    }

    /// Deconstructs a [`RawIdx`] into the underlying [`u32`].
    #[inline]
    pub const fn into_u32(self) -> u32 {
        self.0.get()
    }

    /// Converts [`RawIdx`] into [`Arena`]'s internal index.
    #[inline]
    const fn to_index(self) -> u32 {
        // SAFETY: `RawIdx` wraps a `NonZeroU32`, so its value is at least one,
        // and `unchecked_sub(1)` cannot underflow.
        unsafe { self.0.get().unchecked_sub(1) }
    }

    /// Converts [`Arena`]'s internal index into a [`RawIdx`].
    ///
    /// ## Panics
    ///
    /// Panics if provided index is [`u32::MAX`].
    #[inline]
    const fn from_index(index: u32) -> Self {
        assert!(index != u32::MAX);
        // SAFETY: The assertion above keeps `index` below `u32::MAX`,
        // which is what `from_index_unchecked` requires of its caller.
        unsafe { Self::from_index_unchecked(index) }
    }

    /// Converts [`Arena`]'s internal index into a [`RawIdx`] without checking the value.
    ///
    /// ## Safety
    ///
    /// Caller must ensure that the provided index is less than [`u32::MAX`].
    #[inline]
    const unsafe fn from_index_unchecked(index: u32) -> Self {
        // SAFETY: The caller guarantees `index < u32::MAX`,
        // so `index + 1` does not overflow and cannot be zero.
        RawIdx(unsafe { NonZeroU32::new_unchecked(index.unchecked_add(1)) })
    }
}

impl From<RawIdx> for u32 {
    #[inline]
    fn from(raw: RawIdx) -> u32 {
        raw.into_u32()
    }
}

impl From<RawIdx> for NonZeroU32 {
    #[inline]
    fn from(raw: RawIdx) -> Self {
        raw.0
    }
}

impl From<NonZeroU32> for RawIdx {
    #[inline]
    fn from(idx: NonZeroU32) -> RawIdx {
        RawIdx::new(idx)
    }
}

impl fmt::Debug for RawIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for RawIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The [`Option`]-optimized index of a value allocated in an arena that holds `T`s.
pub struct Idx<T> {
    raw: RawIdx,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Ord for Idx<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T> PartialOrd for Idx<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Idx<T> {}

impl<T> PartialEq for Idx<T> {
    fn eq(&self, other: &Idx<T>) -> bool {
        self.raw == other.raw
    }
}
impl<T> Eq for Idx<T> {}

impl<T> Hash for Idx<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> fmt::Debug for Idx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut type_name = std::any::type_name::<T>();
        if let Some(idx) = type_name.rfind(':') {
            type_name = &type_name[idx + 1..];
        }
        write!(f, "Idx::<{}>({})", type_name, self.raw)
    }
}

impl<T> Idx<T> {
    /// Creates a new index from a [`RawIdx`].
    #[inline]
    pub(crate) const fn from_raw(raw: RawIdx) -> Self {
        Idx {
            raw,
            _ty: PhantomData,
        }
    }

    /// Converts this index into the underlying [`RawIdx`].
    #[inline]
    pub const fn into_raw(self) -> RawIdx {
        self.raw
    }

    /// The position of this id in the arena it was allocated in, counted from zero.
    ///
    /// An id carries the position plus one, which is what makes `Option<Idx<T>>` the size of
    /// an `Idx<T>` ([`RawIdx`]); this is the position itself --- what a dump labels a node
    /// with, and what an algorithm over a graph indexes by.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw.to_index() as usize
    }
}

/// A range of densely allocated arena values.
pub struct IdxRange<T> {
    range: Range<NonZeroU32>,
    _p: PhantomData<T>,
}

impl<T> IdxRange<T> {
    /// Returns whether the index range is empty.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let one = arena.alloc(1);
    /// let two = arena.alloc(2);
    ///
    /// assert!(mlkc_la_arena::IdxRange::from(one..one).is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Returns the start of the index range.
    #[inline]
    pub fn start(&self) -> Idx<T> {
        // SAFETY: An `IdxRange` is built only from `Idx` values,
        // or from a successor its inclusive constructor asserts to stay below `u32::MAX`,
        // so the start bound is below `u32::MAX`.
        Idx::from_raw(unsafe { RawIdx::new_unchecked(self.range.start) })
    }

    /// Returns the end of the index range.
    #[inline]
    pub fn end(&self) -> Idx<T> {
        // SAFETY: An `IdxRange` is built only from `Idx` values,
        // or from a successor its inclusive constructor asserts to stay below `u32::MAX`,
        // so the end bound is below `u32::MAX`.
        Idx::from_raw(unsafe { RawIdx::new_unchecked(self.range.end) })
    }

    pub fn iter(&self) -> IdxRangeIter<T> {
        IdxRangeIter {
            range: self.range.into(),
            _p: PhantomData,
        }
    }
}

impl<T> fmt::Debug for IdxRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("IdxRange::<{}>", std::any::type_name::<T>()))
            .field(&self.range)
            .finish()
    }
}

impl<T> Clone for IdxRange<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for IdxRange<T> {}

impl<T> PartialEq for IdxRange<T> {
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range
    }
}

impl<T> Eq for IdxRange<T> {}

impl<T> Hash for IdxRange<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.range.hash(state);
    }
}

pub struct IdxRangeIter<T> {
    range: std::ops::Range<NonZeroU32>,
    _p: PhantomData<T>,
}

impl<T> Iterator for IdxRangeIter<T> {
    type Item = Idx<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.range.count()
    }

    #[inline]
    fn last(self) -> Option<Idx<T>> {
        self.range
            .last()
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Idx<T>> {
        self.range
            .nth(n)
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn max(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.range
            .max()
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn min(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.range
            .min()
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn is_sorted(self) -> bool
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        self.range.is_sorted()
    }
}

impl<T> DoubleEndedIterator for IdxRangeIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range
            .next_back()
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|raw| Idx::from_raw(unsafe { RawIdx::new_unchecked(raw) }))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.range
            .nth_back(n)
            // SAFETY: The iterator only yields indices from its range,
            // and that range comes from an `IdxRange`, whose bounds are below `u32::MAX`.
            .map(|raw| Idx::from_raw(unsafe { RawIdx::new_unchecked(raw) }))
    }
}

impl<T> ExactSizeIterator for IdxRangeIter<T> {}

impl<T> FusedIterator for IdxRangeIter<T> {}

impl<T> IntoIterator for IdxRange<T> {
    type Item = Idx<T>;
    type IntoIter = IdxRangeIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IdxRangeIter {
            range: self.range.into(),
            _p: PhantomData,
        }
    }
}

impl<T> Clone for IdxRangeIter<T> {
    fn clone(&self) -> Self {
        IdxRangeIter {
            range: self.range.clone(),
            _p: PhantomData,
        }
    }
}

impl<T> fmt::Debug for IdxRangeIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("IdxRangeIter::<{}>", std::any::type_name::<T>()))
            .field(&self.range)
            .finish()
    }
}

impl<T> From<Range<Idx<T>>> for IdxRange<T> {
    #[inline]
    fn from(range: Range<Idx<T>>) -> Self {
        Self {
            range: Range::from(range.start.into_raw().0..range.end.into_raw().0),
            _p: PhantomData,
        }
    }
}

impl<T> From<RangeInclusive<Idx<T>>> for IdxRange<T> {
    #[inline]
    fn from(range: RangeInclusive<Idx<T>>) -> Self {
        let last = range.last.into_raw().into_u32();
        assert!(last < u32::MAX - 1);

        Self {
            range: Range::from(
                range.start.into_raw().0..unsafe {
                    // SAFETY: The assertion above keeps `last` below `u32::MAX - 1`,
                    // so `last + 1` does not overflow and cannot be zero.
                    NonZeroU32::new_unchecked(last.unchecked_add(1))
                },
            ),
            _p: PhantomData,
        }
    }
}

impl<T> From<std::ops::Range<Idx<T>>> for IdxRange<T> {
    /// Creates a new index range
    /// inclusive of the start value and exclusive of the end value.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let a = arena.alloc("a");
    /// let b = arena.alloc("b");
    /// let c = arena.alloc("c");
    /// let d = arena.alloc("d");
    ///
    /// let range = mlkc_la_arena::IdxRange::from(b..d);
    /// assert_eq!(&arena[range], &["b", "c"]);
    /// ```
    #[inline]
    fn from(range: std::ops::Range<Idx<T>>) -> Self {
        Self::from(Range::from(range))
    }
}

impl<T> From<std::ops::RangeInclusive<Idx<T>>> for IdxRange<T> {
    /// Creates a new index range
    /// inclusive of the start value and end value.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let foo = arena.alloc("foo");
    /// let bar = arena.alloc("bar");
    /// let baz = arena.alloc("baz");
    ///
    /// let range = mlkc_la_arena::IdxRange::from(foo..=baz);
    /// assert_eq!(&arena[range], &["foo", "bar", "baz"]);
    ///
    /// let range = mlkc_la_arena::IdxRange::from(foo..=foo);
    /// assert_eq!(&arena[range], &["foo"]);
    /// ```
    #[inline]
    fn from(range: std::ops::RangeInclusive<Idx<T>>) -> Self {
        Self::from(RangeInclusive::from(range))
    }
}

/// Yet another index-based arena.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Arena<T> {
    data: Vec<T>,
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Arena")
            .field("len", &self.len())
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Arena<T> {
    /// Creates a new empty arena.
    ///
    /// ```
    /// let arena: mlkc_la_arena::Arena<i32> = mlkc_la_arena::Arena::new();
    /// assert!(arena.is_empty());
    /// ```
    pub const fn new() -> Arena<T> {
        Arena { data: Vec::new() }
    }

    /// Create a new empty arena with specific capacity.
    ///
    /// ```
    /// let arena: mlkc_la_arena::Arena<i32> = mlkc_la_arena::Arena::with_capacity(42);
    /// assert!(arena.is_empty());
    /// ```
    pub fn with_capacity(capacity: usize) -> Arena<T> {
        Arena {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Empties the arena, removing all contained values.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    ///
    /// arena.alloc(1);
    /// arena.alloc(2);
    /// arena.alloc(3);
    /// assert_eq!(arena.len(), 3);
    ///
    /// arena.clear();
    /// assert!(arena.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns the length of the arena.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// assert_eq!(arena.len(), 0);
    ///
    /// arena.alloc("foo");
    /// assert_eq!(arena.len(), 1);
    ///
    /// arena.alloc("bar");
    /// assert_eq!(arena.len(), 2);
    ///
    /// arena.alloc("baz");
    /// assert_eq!(arena.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns whether the arena contains no elements.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// assert!(arena.is_empty());
    ///
    /// arena.alloc(0.5);
    /// assert!(!arena.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Allocates a new value on the arena, returning the value’s index.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let idx = arena.alloc(50);
    ///
    /// assert_eq!(arena[idx], 50);
    /// ```
    pub fn alloc(&mut self, value: T) -> Idx<T> {
        let idx = self.next_idx();
        self.data.push(value);
        idx
    }

    /// Densely allocates multiple values, returning the values’ index range.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let range = arena.alloc_many(0..4);
    ///
    /// assert_eq!(arena[range], [0, 1, 2, 3]);
    /// ```
    pub fn alloc_many<II: IntoIterator<Item = T>>(&mut self, iter: II) -> IdxRange<T> {
        let start = self.next_idx();
        self.extend(iter);
        let end = self.next_idx();
        IdxRange::from(start..end)
    }

    /// Returns an iterator over the arena’s elements.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let idx1 = arena.alloc(20);
    /// let idx2 = arena.alloc(40);
    /// let idx3 = arena.alloc(60);
    ///
    /// let mut iterator = arena.iter();
    /// assert_eq!(iterator.next(), Some((idx1, &20)));
    /// assert_eq!(iterator.next(), Some((idx2, &40)));
    /// assert_eq!(iterator.next(), Some((idx3, &60)));
    /// ```
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Idx<T>, &T)> + DoubleEndedIterator + Clone {
        self.data.iter().enumerate().map(|(idx, value)| {
            (
                Idx::from_raw(RawIdx(NonZeroU32::new(idx as u32 + 1).unwrap())),
                value,
            )
        })
    }

    /// Returns an iterator over the arena’s mutable elements.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let idx1 = arena.alloc(20);
    ///
    /// assert_eq!(arena[idx1], 20);
    ///
    /// let mut iterator = arena.iter_mut();
    /// *iterator.next().unwrap().1 = 10;
    /// drop(iterator);
    ///
    /// assert_eq!(arena[idx1], 10);
    /// ```
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (Idx<T>, &mut T)> + DoubleEndedIterator {
        self.data.iter_mut().enumerate().map(|(idx, value)| {
            (
                Idx::from_raw(RawIdx(NonZeroU32::new(idx as u32 + 1).unwrap())),
                value,
            )
        })
    }

    /// Returns an iterator over the arena’s values.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let idx1 = arena.alloc(20);
    /// let idx2 = arena.alloc(40);
    /// let idx3 = arena.alloc(60);
    ///
    /// let mut iterator = arena.values();
    /// assert_eq!(iterator.next(), Some(&20));
    /// assert_eq!(iterator.next(), Some(&40));
    /// assert_eq!(iterator.next(), Some(&60));
    /// ```
    pub fn values(&self) -> impl ExactSizeIterator<Item = &T> + DoubleEndedIterator + Clone {
        self.data.iter()
    }

    /// Returns an iterator over the arena’s mutable values.
    ///
    /// ```
    /// let mut arena = mlkc_la_arena::Arena::new();
    /// let idx1 = arena.alloc(20);
    ///
    /// assert_eq!(arena[idx1], 20);
    ///
    /// let mut iterator = arena.values_mut();
    /// *iterator.next().unwrap() = 10;
    /// drop(iterator);
    ///
    /// assert_eq!(arena[idx1], 10);
    /// ```
    pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut T> + DoubleEndedIterator {
        self.data.iter_mut()
    }

    /// Reallocates the arena to make it take up as little space as possible.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Returns the index of the next value allocated on the arena.
    ///
    /// This method should remain private to make creating invalid `Idx`s harder.
    fn next_idx(&self) -> Idx<T> {
        Idx::from_raw(RawIdx::from_index(self.data.len() as u32))
    }
}

impl<T> AsMut<[T]> for Arena<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.data.as_mut()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena { data: Vec::new() }
    }
}

impl<T> Index<Idx<T>> for Arena<T> {
    type Output = T;
    fn index(&self, idx: Idx<T>) -> &T {
        let index = idx.into_raw().to_index() as usize;
        &self.data[index]
    }
}

impl<T> IndexMut<Idx<T>> for Arena<T> {
    fn index_mut(&mut self, idx: Idx<T>) -> &mut T {
        let index = idx.into_raw().to_index() as usize;
        &mut self.data[index]
    }
}

impl<T> Index<IdxRange<T>> for Arena<T> {
    type Output = [T];
    fn index(&self, range: IdxRange<T>) -> &[T] {
        let start = range.start().into_raw().to_index() as usize;
        let end = range.end().into_raw().to_index() as usize;
        &self.data[start..end]
    }
}

impl<T> IndexMut<IdxRange<T>> for Arena<T> {
    fn index_mut(&mut self, range: IdxRange<T>) -> &mut [T] {
        let start = range.start().into_raw().to_index() as usize;
        let end = range.end().into_raw().to_index() as usize;
        &mut self.data[start..end]
    }
}

impl<T> FromIterator<T> for Arena<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Arena {
            data: Vec::from_iter(iter),
        }
    }
}

/// An iterator over the arena’s elements.
pub struct IntoIter<T>(Enumerate<<Vec<T> as IntoIterator>::IntoIter>);

impl<T> Iterator for IntoIter<T> {
    type Item = (Idx<T>, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(index, value)| (Idx::from_raw(RawIdx::from_index(index as u32)), value))
    }
}

impl<T> IntoIterator for Arena<T> {
    type Item = (Idx<T>, T);

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.data.into_iter().enumerate())
    }
}

impl<T> Extend<T> for Arena<T> {
    fn extend<II: IntoIterator<Item = T>>(&mut self, iter: II) {
        for t in iter {
            self.alloc(t);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::{Arena, ArenaIndex, ArenaToken, Idx, IdxRange, RawIdx};

    #[test]
    fn alloc_hands_out_dense_zero_based_ids_in_order() {
        let mut arena = Arena::new();

        let a = arena.alloc(10);
        let b = arena.alloc(20);
        let c = arena.alloc(30);

        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
        assert_eq!(c.index(), 2);
        assert_eq!(a.into_raw().into_u32(), 1);
        assert_eq!(arena[a], 10);
        assert_eq!(arena[b], 20);
        assert_eq!(arena[c], 30);
    }

    #[test]
    fn ids_order_by_allocation_not_by_value() {
        let mut arena = Arena::new();

        let first = arena.alloc(9);
        let second = arena.alloc(1);

        assert!(first < second);
        assert_eq!(arena.values().copied().collect::<Vec<_>>(), [9, 1]);
    }

    #[test]
    fn iteration_follows_allocation_order() {
        let mut arena = Arena::new();
        let a = arena.alloc("a");
        let b = arena.alloc("b");
        let c = arena.alloc("c");

        assert_eq!(arena.iter().collect::<Vec<_>>(), [
            (a, &"a"),
            (b, &"b"),
            (c, &"c")
        ]);
        assert_eq!(arena.values().collect::<Vec<_>>(), [&"a", &"b", &"c"]);
        assert_eq!(arena.iter().rev().collect::<Vec<_>>(), [
            (c, &"c"),
            (b, &"b"),
            (a, &"a")
        ]);
        assert_eq!(arena.iter().len(), arena.len());
    }

    #[test]
    fn iter_mut_and_values_mut_write_through() {
        let mut arena = Arena::new();
        let a = arena.alloc(1);
        let b = arena.alloc(2);

        for (_, value) in arena.iter_mut() {
            *value *= 10;
        }
        for value in arena.values_mut() {
            *value += 1;
        }

        assert_eq!(arena[a], 11);
        assert_eq!(arena[b], 21);
    }

    #[test]
    fn into_iter_yields_ids_and_values_in_order() {
        let mut arena = Arena::new();
        let a = arena.alloc('a');
        let b = arena.alloc('b');

        assert_eq!(arena.into_iter().collect::<Vec<_>>(), [(a, 'a'), (b, 'b')]);
    }

    #[test]
    fn collecting_allocates_densely_in_order() {
        let arena: Arena<i32> = (10..13).collect();

        assert_eq!(arena.len(), 3);
        assert_eq!(arena.values().copied().collect::<Vec<_>>(), [10, 11, 12]);
        assert_eq!(
            arena.iter().map(|(idx, _)| idx.index()).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn extending_appends_without_moving_existing_ids() {
        let mut arena = Arena::new();
        let first = arena.alloc(1);

        arena.extend([2, 3]);

        assert_eq!(arena[first], 1);
        assert_eq!(arena.values().copied().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(
            arena.iter().map(|(idx, _)| idx.index()).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn clearing_empties_the_arena_and_restarts_the_ids() {
        let mut arena = Arena::new();
        arena.alloc(1);
        arena.alloc(2);

        arena.clear();

        assert!(arena.is_empty());
        assert_eq!(arena, Arena::new());

        let fresh = arena.alloc(3);
        assert_eq!(fresh.index(), 0);
        assert_eq!(arena[fresh], 3);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn default_and_with_capacity_start_empty() {
        assert!(Arena::<i32>::new().is_empty());
        assert!(Arena::<i32>::default().is_empty());
        assert!(Arena::<i32>::with_capacity(8).is_empty());
        assert_eq!(Arena::<i32>::with_capacity(8).len(), 0);
    }

    #[test]
    fn alloc_many_returns_the_half_open_range_of_new_ids() {
        let mut arena = Arena::new();
        arena.alloc(99);

        let range = arena.alloc_many([1, 2, 3]);

        assert_eq!(range.start().index(), 1);
        assert_eq!(range.end().index(), 4);
        assert!(!range.is_empty());
        assert_eq!(&arena[range], &[1, 2, 3]);
    }

    #[test]
    fn allocating_no_value_yields_an_empty_range() {
        let mut arena: Arena<i32> = Arena::new();

        let range = arena.alloc_many([]);

        assert!(range.is_empty());
        assert_eq!(range.start(), range.end());
        assert!(arena[range].is_empty());
        assert_eq!(range.iter().count(), 0);
    }

    #[test]
    fn an_index_range_indexes_a_half_open_slice() {
        let mut arena = Arena::new();
        arena.alloc("a");
        let b = arena.alloc("b");
        arena.alloc("c");
        let d = arena.alloc("d");

        let exclusive = IdxRange::from(b..d);

        assert_eq!(&arena[exclusive], &["b", "c"]);
        assert_eq!(exclusive.start(), b);
        assert_eq!(exclusive.end(), d);

        let inclusive = IdxRange::from(b..=d);

        assert_eq!(&arena[inclusive], &["b", "c", "d"]);
        assert_eq!(inclusive.end().index(), d.index() + 1);
        assert_eq!(&arena[IdxRange::from(b..=b)], &["b"]);

        assert!(IdxRange::from(b..b).is_empty());
        assert!(!IdxRange::from(b..=b).is_empty());
    }

    #[test]
    fn an_index_range_can_be_written_through() {
        let mut arena = Arena::new();
        arena.alloc(0);
        let b = arena.alloc(1);
        arena.alloc(2);
        let d = arena.alloc(3);

        let range = IdxRange::from(b..d);

        for value in &mut arena[range] {
            *value *= 10;
        }

        assert_eq!(&arena[range], &[10, 20]);
    }

    #[test]
    fn a_range_iterates_forward_and_backward_over_its_ids() {
        let mut arena = Arena::new();
        arena.alloc(0);
        let b = arena.alloc(1);
        let c = arena.alloc(2);
        let d = arena.alloc(3);

        let range = IdxRange::from(b..d);

        assert_eq!(range.iter().collect::<Vec<_>>(), [b, c]);
        assert_eq!(range.iter().rev().collect::<Vec<_>>(), [c, b]);
        assert_eq!(range.iter().len(), 2);
        assert_eq!(range.into_iter().next_back(), Some(c));
        assert_eq!(range.into_iter().count(), 2);
    }

    #[test]
    fn shrinking_preserves_ids_and_values() {
        let mut arena = Arena::with_capacity(64);
        let a = arena.alloc("a");
        let b = arena.alloc("b");

        arena.shrink_to_fit();

        assert_eq!(arena[a], "a");
        assert_eq!(arena[b], "b");
        assert_eq!(arena.iter().collect::<Vec<_>>(), [(a, &"a"), (b, &"b")]);
    }

    #[test]
    fn as_mut_exposes_the_values_as_a_slice_in_allocation_order() {
        let mut arena = Arena::new();
        let first = arena.alloc(1);
        arena.alloc(2);

        let slice: &mut [i32] = arena.as_mut();
        slice.swap(0, 1);

        assert_eq!(arena[first], 2);
        assert_eq!(arena.values().copied().collect::<Vec<_>>(), [2, 1]);
    }

    #[test]
    fn option_of_an_idx_costs_no_more_than_the_idx() {
        fn assert_niche<T>() {
            assert_eq!(size_of::<Option<Idx<T>>>(), size_of::<Idx<T>>());
        }

        assert_niche::<u32>();
        assert_niche::<String>();
        assert_eq!(size_of::<RawIdx>(), size_of::<u32>());
        assert_eq!(size_of::<Option<RawIdx>>(), size_of::<RawIdx>());
        assert_eq!(size_of::<ArenaToken>(), 0);
    }

    #[test]
    fn a_raw_idx_roundtrips_through_the_u32_conversions() {
        let raw = RawIdx::new(NonZeroU32::new(7).unwrap());

        assert_eq!(raw.into_u32(), 7);
        assert_eq!(u32::from(raw), 7);
        assert_eq!(NonZeroU32::from(raw), NonZeroU32::new(7).unwrap());
        assert_eq!(RawIdx::from(NonZeroU32::new(7).unwrap()), raw);
    }

    #[test]
    fn a_raw_idx_carries_the_position_plus_one() {
        assert_eq!(RawIdx::from_index(0).into_u32(), 1);
        assert_eq!(RawIdx::from_index(0).to_index(), 0);
        assert_eq!(RawIdx::from_index(41).to_index(), 41);
    }

    #[test]
    #[should_panic]
    fn a_raw_idx_refuses_the_value_reserved_for_none() {
        RawIdx::new(NonZeroU32::new(u32::MAX).unwrap());
    }

    #[test]
    #[should_panic]
    fn converting_the_sentinel_from_a_nonzero_u32_panics_too() {
        let _ = RawIdx::from(NonZeroU32::new(u32::MAX).unwrap());
    }

    #[test]
    fn an_idx_roundtrips_through_the_arena_index_trait() {
        let mut arena = Arena::new();
        let idx = arena.alloc("value");

        let raw = ArenaIndex::into_raw(idx);
        let back = <Idx<&str> as ArenaIndex>::from_raw(raw, ArenaToken(()));

        assert_eq!(back, idx);
        assert_eq!(arena[back], "value");
    }
}
