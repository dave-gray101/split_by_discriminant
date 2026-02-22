//! A small utility for partitioning a sequence of items by enum
//! discriminant.
//!
//! This crate helps partition a sequence of items by their enum
//! discriminant. It is useful when you want to gather all values of a
//! particular variant, operate on them, and then return them to the original
//! collection.
//!
//! The primary operation is [`split_by_discriminant`], which accepts any
//! iterable producing values that borrow a type `T` (e.g. `&T`, `&mut T`, or
//! even `T` itself, since `Borrow<T>` is blanket‑implemented for `T`).  It
//! returns a [`SplitByDiscriminant`] helper that exposes grouped references as
//! well as any items that were not matched by the provided discriminants.
//! 
//! For cases where you want to apply custom transformations while
//! partitioning, see the companion function [`map_by_discriminant`], which
//! takes two mapping closures and allows the group and others element types to
//! differ.
//!
//! No traits beyond `std::borrow::Borrow` (used to obtain a `&T` for
//! discriminant computation) are strictly required on the element type.  
//! 
//! The [`indexmap`](https://docs.rs/indexmap/latest/indexmap/) feature toggles which underlying map/set types we use.
//! When the feature is enabled we rely on `indexmap::{IndexMap, IndexSet}`
//! Otherwise, `std::collections::{HashMap, HashSet}` is used.

#[cfg(feature = "indexmap")]
pub(crate) type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(feature = "indexmap")]
pub(crate) type Set<T> = indexmap::IndexSet<T>;

#[cfg(not(feature = "indexmap"))]
pub(crate) type Map<K, V> = std::collections::HashMap<K, V>;
#[cfg(not(feature = "indexmap"))]
pub(crate) type Set<T> = std::collections::HashSet<T>;

use std::mem::Discriminant;
use std::borrow::{Borrow, BorrowMut};

/// Outcome of a discriminant‑based partition operation.
///
/// `SplitByDiscriminant` is generic over the **reference type** `R` produced by
/// the input iterator, so it can represent either mutable or immutable
/// borrows or owned values.  The only requirement on `R` is that it implement
/// [`Borrow<T>`]; this allows the implementation to obtain a `&T` for
/// discriminant computation.  Note that `Borrow<T>` is blanket‑implemented for
/// `T` itself, so `R` may be `T` when you are consuming owned items.
///
/// A lifetime parameter ties the returned references to the original
/// collection when `R` is a reference.  The caller can also write the type
/// explicitly (`SplitByDiscriminant<T, &T>` or `SplitByDiscriminant<T, &mut T>`),
/// but type inference usually makes that unnecessary.
///
/// # Example (immutable slice)
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, SplitByDiscriminant};
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B }
///
/// // compute discriminants on the fly
/// let data = [E::A(1), E::B, E::A(2)];
/// let a_disc = discriminant(&E::A(0));
///
/// let mut split: SplitByDiscriminant<_, &E> =
///     split_by_discriminant(&data[..], &[a_disc]);
/// assert_eq!(split.group(a_disc).unwrap().len(), 2);
///
/// // or store discriminants in constants for reuse
/// const A_DISC: std::mem::Discriminant<E> = discriminant(&E::A(0));
/// const B_DISC: std::mem::Discriminant<E> = discriminant(&E::B);
///
/// let mut split2: SplitByDiscriminant<_, &E> =
///     split_by_discriminant(&data[..], &[A_DISC, B_DISC]);
/// assert_eq!(split2.group(A_DISC).unwrap().len(), 2);
/// ```
/// Result of a discriminant split.
///
/// `G` is the element type stored in each group, and `O` is the element
/// type used for the "others" bucket.  When both are the same – the common
/// case for [`split_by_discriminant`] – the third type parameter can be
/// omitted thanks to the default.
pub struct SplitByDiscriminant<T, G, O = G> {
    groups: Map<Discriminant<T>, Vec<G>>,
    others: Vec<O>,
}


impl<T, G, O> SplitByDiscriminant<T, G, O> {
    /// Deconstruct into the owned collections.
    pub fn into_parts(self) -> (Map<Discriminant<T>, Vec<G>>, Vec<O>) {
        (self.groups, self.others)
    }

    /// Access the stored group for a discriminant.
    pub fn group(&mut self, id: Discriminant<T>) -> Option<&Vec<G>> {
        self.groups.get(&id)
    }
}

// implement extract only when we actually have mutable access to the inner T
// implement extract only when the group element type supports mutable
// borrowing of `T`.  the "others" type is irrelevant here.
impl<T, G, O> SplitByDiscriminant<T, G, O>
where
    G: BorrowMut<T>,
{
    /// See the earlier documentation; this method is only available when the
    /// group element type supports mutable borrowing.
    pub fn extract<U>(&mut self, id: Discriminant<T>) -> Option<Vec<&mut U>>
    where
        T: Extract<U>,
    {
        if let Some(vec) = self.groups.get_mut(&id) {
            let mut out = Vec::with_capacity(vec.len());
            for item in vec.iter_mut() {
                if let Some(u) = item.borrow_mut().extract() {
                    out.push(u);
                }
            }
            Some(out)
        } else {
            None
        }
    }
}

/// A helper trait used by [`SplitByDiscriminant::extract`].
///
/// `Extract<U>` defines how to obtain a `&mut U` from a `&mut T`.  When `T`
/// is an enum, the implementation typically checks for a particular variant
/// and returns a reference to the contained value.  Returns `None` if the
/// conversion is not possible.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::Extract;
///
/// enum E { A(i32), B(String) }
///
/// impl Extract<i32> for E {
///     fn extract(&mut self) -> Option<&mut i32> {
///         if let E::A(v) = self { Some(v) } else { None }
///     }
/// }
///
/// // now you can call `split.extract(a_disc)` where `a_disc` is the
/// // discriminant for `E::A`.
/// ```
pub trait Extract<U> {
    fn extract(&mut self) -> Option<&mut U>;
}

/// Partition a sequence of elements into groups keyed by enum discriminant.
///
/// The `items` argument may be any iterator whose `Item` type is some value
/// `R` implementing `Borrow<T>` (typically `&T` or `&mut T`, but also `T`
/// itself).  Because of the blanket implementation of `Borrow<T>` for `T`,
/// owned iterators such as `Vec<T>::into_iter()` work seamlessly, allowing you
/// to take ownership of the elements.  Alternatively, pass `&[T]`, `&mut
/// Vec<T>`, or any other container that yields references, and the return type
/// will reflect the borrow kind.  When `R` is an immutable reference, helpers
/// like [`SplitByDiscriminant::extract`] are omitted via trait bounds.
///
/// Examples of accepted inputs:
///
/// * `&mut [T]`, `&mut Vec<T>` (yields `&mut T` items)
/// * `&[T]`, `&Vec<T>` (yields `&T` items)
/// * `Vec<T>::into_iter()` (yields `T` items)
///
/// A bare `[T]` cannot be passed by value because it is unsized; you always
/// borrow it.
///
/// The `kinds` argument is any iterable of values convertible to
/// `Discriminant<T>`; duplicates are discarded internally.
///
/// # Examples
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, Extract};
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B(String), C }
///
/// impl Extract<i32> for E {
///     fn extract(&mut self) -> Option<&mut i32> {
///         if let E::A(v) = self { Some(v) } else { None }
///     }
/// }
///
/// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
/// let a_disc = discriminant(&E::A(0));
/// let b_disc = discriminant(&E::B(String::new()));
/// // you can pre‑compute and stash these in `const`s for later use
/// // const A_DISC: std::mem::Discriminant<E> = discriminant(&E::A(0));
/// // const B_DISC: std::mem::Discriminant<E> = discriminant(&E::B(String::new()));
///
/// // can pass a mutable slice
/// let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
/// assert_eq!(split.group(a_disc).unwrap().len(), 2);
///
/// // or a mutable Vec
/// let mut vec = vec![E::A(3), E::C];
/// let mut split2 = split_by_discriminant(&mut vec, &[a_disc]);
/// assert_eq!(split2.group(a_disc).unwrap().len(), 1);
///
/// // you can even consume an owned collection; here `R` is `E` itself
/// // (Borrow<E> is implemented for E), so the iterator yields `E` values.
/// let owned = vec![E::A(4), E::B(String::new())];
/// let mut split3 = split_by_discriminant(owned.into_iter(), &[a_disc]);
/// assert_eq!(split3.group(a_disc).unwrap().len(), 1);
/// ```
///
/// The return type reflects the borrow kind of the iterator.  Passing a
/// mutable container yields `SplitByDiscriminant<T, &mut T>`, an immutable
/// borrow gives `SplitByDiscriminant<T, &T>`, and an owning iterator results in
/// `SplitByDiscriminant<T, T>`.
/// 
/// # Example (custom mapping)
///
/// ```rust
/// use split_by_discriminant::map_by_discriminant;
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B };
/// let a_disc = discriminant(&E::A(0));
/// let b_disc = discriminant(&E::B);
///
/// let data = [E::A(1), E::B];
/// let mut split = map_by_discriminant(&data[..], &[a_disc, b_disc],
///     |e| format!("match:{:?}", e),
///     |e| format!("other:{:?}", e),
/// );
/// assert_eq!(split.group(a_disc).unwrap()[0], "match:A(1)");
/// ```
///
/// The two closures control what happens to matched vs unmatched items
/// allowing the caller to transform values.
///
/// `map_by_discriminant` returns a `SplitByDiscriminant` where the
/// group and other element types may differ; this is what motivated the
/// additional type parameter on the struct itself.  The implementation is
/// identical to the old `split_internal` helper but inlined here so there is
/// only a single public function.
pub fn map_by_discriminant<T, I, K, R, U, V, M, N>(
    items: I,
    kinds: K,
    mut map_match: M,
    mut map_other: N,
) -> SplitByDiscriminant<T, U, V>
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
        .map(|k| k.borrow().clone())
        .collect();

    let mut groups: Map<Discriminant<T>, Vec<U>> = Map::new();
    let mut others: Vec<V> = Vec::new();

    for item in items.into_iter() {
        let d = std::mem::discriminant(item.borrow());
        if wanted.contains(&d) {
            groups.entry(d).or_default().push(map_match(item));
        } else {
            others.push(map_other(item));
        }
    }

    SplitByDiscriminant { groups, others }
}

/// Partition a sequence of elements into groups keyed by enum discriminant.
///
/// This is the simplest function in the library; it simply returns
/// `SplitByDiscriminant<T, R>` where the two element types coincide.
/// The bound `R: Borrow<T>` is required for the discriminant computation.
pub fn split_by_discriminant<T, I, K, R>(
    items: I,
    kinds: K,
) -> SplitByDiscriminant<T, R>
where
    I: IntoIterator<Item = R>,
    R: Borrow<T>,
    K: IntoIterator,
    K::Item: Borrow<Discriminant<T>>,
{
    map_by_discriminant(items, kinds, |r| r, |r| r)
}


impl<T, R> SplitByDiscriminant<T, R>
where
    R: Borrow<T>,
{
    /// Transform each group all at once, consuming `self`.
    ///
    /// The provided function is given ownership of the entire `Vec<R>` for
    /// each discriminant; it may convert them to some other representation.
    /// This is primarily a convenience for callers who want an immediate
    /// post‑processing step without manually iterating the map returned by
    /// [`SplitByDiscriminant::into_parts`].
    pub fn map_groups<U, F>(self, mut f: F) -> Map<Discriminant<T>, U>
    where
        F: FnMut(Vec<R>) -> U,
    {
        self.groups
            .into_iter()
            .map(|(k, v)| (k, f(v)))
            .collect()
    }

    /// Apply a transformation to the "others" vector.
    pub fn map_others<U, F>(self, f: F) -> U
    where
        F: FnOnce(Vec<R>) -> U,
    {
        f(self.others)
    }
}


#[cfg(test)]
mod tests;
