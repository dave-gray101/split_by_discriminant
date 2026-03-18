use std::mem::Discriminant;
use std::borrow::{Borrow, BorrowMut};

use crate::{Map, Set};

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
    others: Vec<O>,
}

impl<T, G, O> DiscriminantMap<T, G, O> {
    /// Deconstruct into the owned collections.
    pub fn into_parts(self) -> (Map<Discriminant<T>, Vec<G>>, Vec<O>) {
        (self.entries, self.others)
    }

    /// Borrow the unmatched items.
    pub fn others(&self) -> &[O] {
        &self.others
    }

    /// Access the stored group for a discriminant.
    pub fn get(&self, id: Discriminant<T>) -> Option<&[G]> {
        self.entries.get(&id).map(Vec::as_slice)
    }

    /// Mutably borrow the entries for `id` as a mutable slice.
    ///
    /// Returns `Option<&mut [G]>`.  Slice methods that reorder or mutate
    /// elements in place are all available.  Structural modifications
    /// (`push`, `drain`, `retain`) are intentionally unavailable to preserve
    /// the discriminant invariant.  Use [`remove`](Self::remove) when
    /// structural modification is needed.
    pub fn get_mut(&mut self, id: Discriminant<T>) -> Option<&mut [G]> {
        self.entries.get_mut(&id).map(Vec::as_mut_slice)
    }

    /// Borrow references to inner fields of each group element using an inline
    /// closure — no extractor struct, no selector type, no turbofish.
    ///
    /// The return type `Vec<&mut U>` is fully determined by the type annotation
    /// on the binding; `U` is inferred from context exactly as in v0.x.
    ///
    /// ```rust
    /// use split_by_discriminant::split_by_discriminant;
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
    ///
    /// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let mut map = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    ///
    /// // U = i32 inferred from Vec<&mut i32> — no turbofish:
    /// let ints: Vec<&mut i32> = map
    ///     .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    ///     .unwrap();
    ///
    /// // U = String inferred independently — same method, different U:
    /// let strings: Vec<&mut String> = map
    ///     .extract_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
    ///     .unwrap();
    /// ```
    ///
    /// **Lifetime constraint:** `U` must not contain borrows shorter than
    /// `'static` (e.g. `i32`, `String`, `IpAddr` are all fine; a field
    /// `&str` is not — use [`SplitWithExtractor::extract_gat`] for those).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to the
    /// split function.
    pub fn extract_with<'s, U, F>(&'s mut self, id: Discriminant<T>, mut f: F) -> Option<Vec<&'s mut U>>
    where
        G: BorrowMut<T>,
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        let slice = self.entries.get_mut(&id)?;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter_mut() {
            if let Some(u) = f(item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Remove and return the owned group for `id`, if present.
    ///
    /// Unlike [`get`](Self::get) and [`get_mut`](Self::get_mut), which
    /// reborrow and shorten any inner lifetime, `remove` **moves** the
    /// `Vec<G>` out of the map and returns it with its original lifetime
    /// intact.  A second call for the same discriminant returns `None`.
    pub fn remove(&mut self, id: Discriminant<T>) -> Option<Vec<G>> {
        self.entries.remove(&id)
    }

    /// Remove the group for `id` and map each element through `f`.
    ///
    /// Returns `None` when `id` was not among the discriminants passed to the
    /// split function.
    pub fn remove_mapped<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<U>>
    where
        F: FnMut(G) -> U,
    {
        Some(self.entries.remove(&id)?.into_iter().map(f).collect())
    }

    /// Remove the group for `id`, apply `f` to each element **by value**, and
    /// collect the `Some` results (filter-map semantics).
    ///
    /// Items for which `f` returns `None` are skipped.  Returns `None` when
    /// `id` was not among the discriminants passed to the split function.
    pub fn remove_with<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<U>>
    where
        F: FnMut(G) -> Option<U>,
    {
        Some(
            self.entries
                .remove(&id)?
                .into_iter()
                .filter_map(f)
                .collect(),
        )
    }

    /// Remove and return the others vector.
    ///
    /// The vector is replaced with an empty `Vec` in-place.
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
    type Item = (&'a Discriminant<T>, &'a mut [G]);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.entries.iter_mut().map(|(k, v)| (k, v.as_mut_slice())))
    }
}

// ── Free functions ────────────────────────────────────────────────────────────

/// Partition a sequence of elements into groups keyed by enum discriminant,
/// applying separate transformations to matched and unmatched items.
///
/// `items` may be any iterator whose `Item` type `R` implements `Borrow<T>`.
/// `kinds` lists the discriminants to split on; duplicates are ignored.
pub fn map_by_discriminant<T, I, K, R, U, V, M, N>(
    items: I,
    kinds: K,
    mut map_match: M,
    mut map_other: N,
) -> DiscriminantMap<T, U, V>
where
    I: IntoIterator<Item = R>,
    R: Borrow<T>,
    K: IntoIterator,
    K::Item: Borrow<Discriminant<T>>,
    M: FnMut(R) -> U,
    N: FnMut(R) -> V,
{
    let wanted: Set<Discriminant<T>> = kinds
        .into_iter()
        .map(|k| *k.borrow())
        .collect();

    let mut entries = Map::<Discriminant<T>, Vec<U>>::new();
    let mut others = Vec::<V>::new();
    for item in items.into_iter() {
        let d = std::mem::discriminant(item.borrow());
        if wanted.contains(&d) {
            entries.entry(d).or_default().push(map_match(item));
        } else {
            others.push(map_other(item));
        }
    }

    DiscriminantMap { entries, others }
}

/// Partition a sequence of elements into groups keyed by enum discriminant.
///
/// `items` may be any iterator whose `Item` type `R` implements `Borrow<T>`.
/// `kinds` lists the discriminants to split on; duplicates are ignored.
///
/// # Examples
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, DiscriminantMap};
/// use std::mem::discriminant;
///
/// #[derive(Debug)] enum E { A(i32), B(String), C }
/// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
/// let a_disc = discriminant(&E::A(0));
///
/// let mut map = split_by_discriminant(&mut data[..], &[a_disc]);
/// assert_eq!(map.get(a_disc).unwrap().len(), 2);
/// assert_eq!(map.others().len(), 2); // B and C
/// ```
pub fn split_by_discriminant<T, I, K, R>(
    items: I,
    kinds: K,
) -> DiscriminantMap<T, R>
where
    I: IntoIterator<Item = R>,
    R: Borrow<T>,
    K: IntoIterator,
    K::Item: Borrow<Discriminant<T>>,
{
    map_by_discriminant(items, kinds, |r| r, |r| r)
}
