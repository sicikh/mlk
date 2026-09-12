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

impl RawIdx {
    /// Maximum value of [`RawIdx`].
    const MAX: u32 = u32::MAX - 1;
    /// Minimum value of [`RawIdx`].
    const MIN: u32 = 1;
}

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
        unsafe { Self::from_index_unchecked(index) }
    }

    /// Converts [`Arena`]'s internal index into a [`RawIdx`] without checking the value.
    ///
    /// ## Safety
    ///
    /// Caller must ensure that the provided index is less than [`u32::MAX`].
    #[inline]
    const unsafe fn from_index_unchecked(index: u32) -> Self {
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
        Idx::from_raw(unsafe { RawIdx::new_unchecked(self.range.start) })
    }

    /// Returns the end of the index range.
    #[inline]
    pub fn end(&self) -> Idx<T> {
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
            .map(|index| Idx::from_raw(unsafe { RawIdx::new_unchecked(index) }))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Idx<T>> {
        self.range
            .nth(n)
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
            .map(|raw| Idx::from_raw(unsafe { RawIdx::new_unchecked(raw) }))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.range
            .nth_back(n)
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
