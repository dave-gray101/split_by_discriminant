//! Consuming removal operations: `remove_*` for both [`DiscriminantMap`] and [`SplitWithExtractor`].
//!
//! All methods here remove groups from the map.  The corresponding [`take_*`](crate::take)
//! methods handle extractor-driven consuming extraction.

use std::mem::Discriminant;

use crate::{DiscriminantMap, Map};
use crate::SplitWithExtractor;

// ── DiscriminantMap: remove_* ─────────────────────────────────────────────────

impl<T, G, O> DiscriminantMap<T, G, O> {
    /// Remove and return the owned group for `id`, if present.
    ///
    /// Unlike [`get`](Self::get) and [`get_mut`](Self::get_mut), which reborrow
    /// and shorten any inner lifetime, `remove` **moves** the `Vec<G>` out and
    /// returns it with its original lifetime intact.  A second call for the same
    /// discriminant returns `None`.
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

    /// Remove and return the owned groups for a slice of discriminants.
    ///
    /// Returns a map containing groups for discriminants that were present.
    /// A second call for the same discriminant returns nothing for that key.
    pub fn remove_multiple(&mut self, ids: &[Discriminant<T>]) -> Map<Discriminant<T>, Vec<G>> {
        ids.iter()
            .filter_map(|id| self.entries.remove(id).map(|group| (*id, group)))
            .collect()
    }

    /// Remove the groups for a slice of discriminants and map each element through `f`.
    ///
    /// Returns a map of transformed groups for discriminants that were present.
    pub fn remove_multiple_mapped<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        mut f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        F: FnMut(G) -> U,
    {
        ids.iter()
            .filter_map(|id| {
                self.entries
                    .remove(id)
                    .map(|group| (*id, group.into_iter().map(&mut f).collect()))
            })
            .collect()
    }

    /// Remove the groups for a slice of discriminants, apply `f` to each element, and
    /// collect the `Some` results (filter-map semantics).
    ///
    /// Returns a map of filtered and transformed groups for discriminants that were present.
    pub fn remove_multiple_with<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        mut f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        F: FnMut(G) -> Option<U>,
    {
        ids.iter()
            .filter_map(|id| {
                self.entries
                    .remove(id)
                    .map(|group| (*id, group.into_iter().filter_map(&mut f).collect()))
            })
            .collect()
    }
}

// ── SplitWithExtractor: remove_* delegations ─────────────────────────────────

impl<T, G, O, E> SplitWithExtractor<T, G, O, E> {
    /// Remove and return the owned group for `id`.
    pub fn remove(&mut self, id: Discriminant<T>) -> Option<Vec<G>> {
        self.inner.remove(id)
    }

    /// Remove the group for `id` and map each element through `f`.
    pub fn remove_mapped<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<U>>
    where
        F: FnMut(G) -> U,
    {
        self.inner.remove_mapped(id, f)
    }

    /// Remove the group for `id`, apply `f` to each element, collect `Some` results.
    pub fn remove_with<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<U>>
    where
        F: FnMut(G) -> Option<U>,
    {
        self.inner.remove_with(id, f)
    }

    /// Remove and return the others vector.
    pub fn remove_others(&mut self) -> Vec<O> {
        self.inner.remove_others()
    }

    /// Remove and return the owned groups for a slice of discriminants.
    pub fn remove_multiple(&mut self, ids: &[Discriminant<T>]) -> Map<Discriminant<T>, Vec<G>> {
        self.inner.remove_multiple(ids)
    }

    /// Remove the groups for a slice of discriminants and map each element through `f`.
    pub fn remove_multiple_mapped<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        F: FnMut(G) -> U,
    {
        self.inner.remove_multiple_mapped(ids, f)
    }

    /// Remove the groups for a slice of discriminants, apply `f` to each element,
    /// and collect the `Some` results.
    pub fn remove_multiple_with<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        F: FnMut(G) -> Option<U>,
    {
        self.inner.remove_multiple_with(ids, f)
    }
}
