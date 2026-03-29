//! [`SplitWithExtractor`] struct definition and construction methods.

use std::mem::Discriminant;

use crate::DiscriminantMap;

/// A [`DiscriminantMap`] with an extractor value bound up front.
///
/// This wrapper carries four generic parameters:
///
/// * `T` – the enum type whose discriminant keys the groups.
/// * `G` – element type stored in each group.
/// * `O` – element type for the others bucket (defaults to `G`).
/// * `E` – an extractor value implementing [`ExtractFrom<T, S>`](crate::ExtractFrom) for one or
///   more selector types `S`.
///
/// Construct with [`SplitWithExtractor::new`] after calling
/// [`crate::split_by_discriminant`].
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
/// use std::mem::discriminant;
///
/// #[derive(Debug)] pub enum MyEnum { A(i32), B(String) }
///
/// struct MyExtractor;
/// pub struct SelectA;
/// pub struct SelectB;
///
/// impl ExtractFrom<MyEnum, SelectA> for MyExtractor {
///     type Output<'a> = &'a mut i32;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
/// impl ExtractFrom<MyEnum, SelectB> for MyExtractor {
///     type Output<'a> = &'a mut String;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::B(s) = t { Some(s) } else { None }
///     }
/// }
///
/// let mut data = vec![MyEnum::A(1), MyEnum::B("hi".into()), MyEnum::A(2)];
/// let a_disc = discriminant(&MyEnum::A(0));
/// let b_disc = discriminant(&MyEnum::B(String::new()));
///
/// let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
/// let mut extractor = SplitWithExtractor::new(split, MyExtractor);
///
/// { let v: Vec<&mut i32>    = extractor.as_mut_with::<SelectA>(a_disc).unwrap(); assert_eq!(v.len(), 2); }
/// { let v: Vec<&mut String> = extractor.as_mut_with::<SelectB>(b_disc).unwrap(); assert_eq!(v.len(), 1); }
/// ```
pub struct SplitWithExtractor<T, G, O, E> {
    pub(crate) inner: DiscriminantMap<T, G, O>,
    pub(crate) extractor: E,
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E> {
    /// Wrap a split and an extractor together.
    pub fn new(split: DiscriminantMap<T, G, O>, extractor: E) -> Self {
        SplitWithExtractor { inner: split, extractor }
    }

    /// Unwrap back to the underlying [`DiscriminantMap`].
    pub fn into_inner(self) -> DiscriminantMap<T, G, O> {
        self.inner
    }

    /// Borrow the unmatched items.
    pub fn others(&self) -> &[O] {
        self.inner.others()
    }

    /// Mutably borrow the unmatched items.
    pub fn others_mut(&mut self) -> &mut [O] {
        self.inner.others_mut()
    }

    /// Access the stored group for a discriminant as a shared slice.
    pub fn get(&self, id: Discriminant<T>) -> Option<&[G]> {
        self.inner.get(id)
    }

    /// Mutably borrow the group for `id` as a [`GroupMut`](crate::GroupMut).
    ///
    /// See [`DiscriminantMap::get_mut`](crate::DiscriminantMap::get_mut) for
    /// the invariant documentation.
    pub fn get_mut(&mut self, id: Discriminant<T>) -> Option<crate::GroupMut<'_, G>> {
        self.inner.get_mut(id)
    }

    /// Call `f` once for each discriminant in `ids` that is present in the map,
    /// passing the corresponding [`GroupMut`](crate::GroupMut).
    ///
    /// Duplicates in `ids` are silently ignored — each group is visited at most
    /// once.  Delegates to
    /// [`DiscriminantMap::for_each_group_mut`](crate::DiscriminantMap::for_each_group_mut).
    pub fn for_each_group_mut<F>(&mut self, ids: &[Discriminant<T>], f: F)
    where
        F: FnMut(Discriminant<T>, crate::GroupMut<'_, G>),
    {
        self.inner.for_each_group_mut(ids, f);
    }
}

// ── IntoIterator delegation ───────────────────────────────────────────────────

impl<T, G, O, E> IntoIterator for SplitWithExtractor<T, G, O, E> {
    type Item = (Discriminant<T>, Vec<G>);
    type IntoIter = <DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T, G, O, E> IntoIterator for &'a SplitWithExtractor<T, G, O, E> {
    type Item = (&'a Discriminant<T>, &'a [G]);
    type IntoIter = <&'a DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

impl<'a, T, G, O, E> IntoIterator for &'a mut SplitWithExtractor<T, G, O, E> {
    type Item = (&'a Discriminant<T>, crate::GroupMut<'a, G>);
    type IntoIter = <&'a mut DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.inner).into_iter()
    }
}
