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
//! To extract inner values without closures at the call site, wrap the split
//! in a [`SplitWithExtractor`] and implement [`ExtractFrom`] on a local extractor
//! type.  This pattern remains orphan-rule–safe even when the enum comes from
//! an external crate that you cannot modify.
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

mod split;
mod extractor;

pub use split::{SplitByDiscriminant, split_by_discriminant, map_by_discriminant};
pub use extractor::{ExtractFrom, SplitWithExtractor};

