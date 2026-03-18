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
mod extractor_traits;
mod extractor_impls;

#[cfg(feature = "proc_macro")]
pub mod proc_macro;

pub use split::{DiscriminantMap, split_by_discriminant, map_by_discriminant};
pub use extractor_traits::{SimpleExtractFrom, VariantExtractFrom, ExtractFrom, TakeFrom, SplitWithExtractor};

