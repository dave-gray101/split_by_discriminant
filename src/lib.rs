#![doc = include_str!("../README.md")]

#[cfg(feature = "indexmap")]
pub(crate) type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(feature = "indexmap")]
pub(crate) type Set<T> = indexmap::IndexSet<T>;

#[cfg(not(feature = "indexmap"))]
pub(crate) type Map<K, V> = std::collections::HashMap<K, V>;
#[cfg(not(feature = "indexmap"))]
pub(crate) type Set<T> = std::collections::HashSet<T>;

mod discriminant_map;
mod entry_points;
mod extractor_traits;
mod group_mut;
mod ref_access;
mod removal;
mod split_with_extractor;
mod take;

#[cfg(feature = "proc_macro")]
pub mod proc_macro;

pub use discriminant_map::DiscriminantMap;
pub use entry_points::{split_by_discriminant, map_by_discriminant};
pub use extractor_traits::{SimpleExtractFrom, VariantExtractFrom, ExtractFrom, TakeFrom,
                           SimpleReadFrom, VariantReadFrom, ReadFrom};
pub use group_mut::GroupMut;
pub use split_with_extractor::SplitWithExtractor;

