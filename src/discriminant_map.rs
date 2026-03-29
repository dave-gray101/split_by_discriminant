//! [`DiscriminantMap`] struct and its core structural-access methods.

use std::mem::Discriminant;

use crate::{GroupMut, Map, Set};

/// Outcome of a discriminant-based partition operation.
///
/// `DiscriminantMap` is generic over three types:
///
/// * `T` – the enum (or other type) whose [`Discriminant`] keys the groups.
/// * `G` – the element type stored in each matching group.  Usually the
///   input iterator's item type (`&T`, `&mut T`, or `T`), but `map_by_discriminant`
///   lets you transform it to any `U`.
/// * `O` – the element type for unmatched items ("others").  Defaults to `G`
///   for the common case.
///
/// # Examples
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, DiscriminantMap};
/// use std::mem::discriminant;
///
/// #[derive(Debug, PartialEq)] enum E { A(i32), B }
/// let data = [E::A(1), E::B, E::A(2)];
/// let a_disc = discriminant(&E::A(0));
///
/// let split: DiscriminantMap<_, &E> = split_by_discriminant(&data[..], &[a_disc]);
/// assert_eq!(split.get(a_disc).unwrap().len(), 2);
/// ```
pub struct DiscriminantMap<T, G, O = G> {
    pub(crate) entries: Map<Discriminant<T>, Vec<G>>,
    pub(crate) others: Vec<O>,
}

impl<T, G, O> DiscriminantMap<T, G, O> {
    /// Construct from pre-built collections.  Used internally by the entry-point functions.
    pub(crate) fn build(entries: Map<Discriminant<T>, Vec<G>>, others: Vec<O>) -> Self {
        DiscriminantMap { entries, others }
    }

    /// Deconstruct into the owned collections.
    pub fn into_parts(self) -> (Map<Discriminant<T>, Vec<G>>, Vec<O>) {
        (self.entries, self.others)
    }

    /// Borrow the unmatched items.
    pub fn others(&self) -> &[O] {
        &self.others
    }

    /// Mutably borrow the unmatched items.
    pub fn others_mut(&mut self) -> &mut [O] {
        &mut self.others
    }

    /// Access the stored group for a discriminant.
    pub fn get(&self, id: Discriminant<T>) -> Option<&[G]> {
        self.entries.get(&id).map(Vec::as_slice)
    }

    /// Access the stored groups for a slice of discriminants, returning a map of the present ones.
    pub fn get_multiple(&self, ids: &[Discriminant<T>]) -> Map<&Discriminant<T>, &[G]> {
        ids.iter()
            .filter_map(|id| {
                self.entries
                    .get_key_value(id)
                    .map(|(k, v)| (k, v.as_slice()))
            })
            .collect()
    }

    /// Mutably borrow the entries for `id` as a [`GroupMut`].
    ///
    /// [`GroupMut`] exposes reordering and iteration while intentionally
    /// omitting `IndexMut` and structural modifications (`push`, `drain`,
    /// `retain`).  Use [`remove`](crate::DiscriminantMap::remove) when
    /// structural modification is needed.
    ///
    /// # Invariant
    ///
    /// Elements in the returned group must remain the same variant.  Using
    /// `iter_mut()` to double-dereference and replace an element's variant
    /// (`**item = WrongVariant(...)`) will silently break the discriminant
    /// invariant.  For guaranteed field-only access use [`map_as_mut`](Self::map_as_mut)
    /// or the `as_mut_*` methods on [`SplitWithExtractor`](crate::SplitWithExtractor).
    pub fn get_mut(&mut self, id: Discriminant<T>) -> Option<GroupMut<'_, G>> {
        self.entries.get_mut(&id).map(|v| GroupMut::new(v.as_mut_slice()))
    }

    /// Apply a closure to each requested group in turn, one group at a time.
    ///
    /// This is the replacement for the removed `get_multiple_mut`.  The
    /// callback pattern ensures that only one [`GroupMut`] is live at a time,
    /// eliminating the possibility of holding simultaneously-live mutable
    /// borrows from different groups and swapping elements between them.
    ///
    /// Groups not present in the map (discriminants that were never requested
    /// during splitting) are silently skipped.  Duplicate entries in `ids` are
    /// deduplicated — each present group is visited at most once.
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::split_by_discriminant;
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
    /// let mut data = [E::A(1), E::A(2), E::B("hi".into())];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let mut map = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    ///
    /// map.for_each_group_mut(&[a_disc, b_disc], |disc, mut group| {
    ///     println!("disc {:?}: {} items", disc, group.len());
    /// });
    /// ```
    pub fn for_each_group_mut<F>(&mut self, ids: &[Discriminant<T>], mut f: F)
    where
        F: FnMut(Discriminant<T>, GroupMut<'_, G>),
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        for (id, v) in self.entries.iter_mut() {
            if wanted.contains(id) {
                f(*id, GroupMut::new(v.as_mut_slice()));
            }
        }
    }

    /// Remove and return the others vector, replacing it with an empty `Vec` in-place.
    pub fn remove_others(&mut self) -> Vec<O> {
        std::mem::take(&mut self.others)
    }

    /// Apply a transformation to each group, consuming `self`.
    pub fn map_all<U, F>(self, mut f: F) -> Map<Discriminant<T>, U>
    where
        F: FnMut(Vec<G>) -> U,
    {
        self.entries.into_iter().map(|(k, v)| (k, f(v))).collect()
    }

    /// Apply a transformation to the others vector, consuming `self`.
    pub fn map_others<U, F>(self, f: F) -> U
    where
        F: FnOnce(Vec<O>) -> U,
    {
        f(self.others)
    }
}

// ── IntoIterator impls ────────────────────────────────────────────────────────

impl<T, G, O> IntoIterator for DiscriminantMap<T, G, O> {
    type Item = (Discriminant<T>, Vec<G>);
    type IntoIter = <Map<Discriminant<T>, Vec<G>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, T, G, O> IntoIterator for &'a DiscriminantMap<T, G, O> {
    type Item = (&'a Discriminant<T>, &'a [G]);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.entries.iter().map(|(k, v)| (k, v.as_slice())))
    }
}

impl<'a, T, G, O> IntoIterator for &'a mut DiscriminantMap<T, G, O> {
    type Item = (&'a Discriminant<T>, GroupMut<'a, G>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.entries
                .iter_mut()
                .map(|(k, v)| (k, GroupMut::new(v.as_mut_slice()))),
        )
    }
}
