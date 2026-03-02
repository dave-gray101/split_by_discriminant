#![doc = include_str!("../README.md")]

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
pub use extractor::{ExtractFrom, TakeFrom, SplitWithExtractor};

