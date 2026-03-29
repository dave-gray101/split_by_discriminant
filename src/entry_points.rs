//! [`split_by_discriminant`] and [`map_by_discriminant`] entry-point functions.

use std::borrow::Borrow;
use std::mem::Discriminant;

use crate::{DiscriminantMap, Map, Set};

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

    DiscriminantMap::build(entries, others)
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
