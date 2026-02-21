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
//! No traits beyond `std::borrow::Borrow` (used to obtain a `&T` for
//! discriminant computation) are strictly required on the element type.  
//! 


// `indexmap` feature toggles which underlying map/set types we use.  When
// the feature is enabled we rely on `indexmap::{IndexMap, IndexSet}`; the
// rest of the code only sees the `Map`/`Set` aliases below.  Keeping the
// aliases at crate root (and `pub(crate)` so the unit tests can reach them)
// makes it easy to compile a small sanity check later.

#[cfg(feature = "indexmap")]
pub(crate) type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(feature = "indexmap")]
pub(crate) type Set<T> = indexmap::IndexSet<T>;

#[cfg(not(feature = "indexmap"))]
pub(crate) type Map<K, V> = std::collections::HashMap<K, V>;
#[cfg(not(feature = "indexmap"))]
pub(crate) type Set<T> = std::collections::HashSet<T>;

use std::mem::{Discriminant, discriminant};
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
pub struct SplitByDiscriminant<T, R>
where
    R: Borrow<T>,
{
    groups: Map<Discriminant<T>, Vec<R>>,
    others: Vec<R>,
}


impl<T, R> SplitByDiscriminant<T, R>
where
    R: Borrow<T>,
{
    /// Deconstruct into the owned collections.
    pub fn into_parts(self) -> (Map<Discriminant<T>, Vec<R>>, Vec<R>) {
        (self.groups, self.others)
    }

    /// Access the stored group for a discriminant.
    pub fn group(&mut self, id: Discriminant<T>) -> Option<&Vec<R>> {
        self.groups.get(&id)
    }
}

// implement extract only when we actually have mutable access to the inner T
impl<T, R> SplitByDiscriminant<T, R>
where
    R: BorrowMut<T>,
{
    /// See the earlier documentation; this method is only available when the
    /// reference type supports mutable borrowing.
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
/// like [`extract`] are omitted via trait bounds.
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
    let wanted: Set<Discriminant<T>> = kinds
        .into_iter()
        .map(|k| k.borrow().clone())
        .collect();

    let mut groups: Map<Discriminant<T>, Vec<R>> = Map::new();
    let mut others = Vec::new();

    for item in items.into_iter() {
        let d = discriminant(item.borrow());
        if wanted.contains(&d) {
            groups.entry(d).or_default().push(item);
        } else {
            others.push(item);
        }
    }

    SplitByDiscriminant { groups, others }
}


#[cfg(test)]
mod tests;
