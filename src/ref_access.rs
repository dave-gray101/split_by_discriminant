//! Non-removing reference access: `as_ref_*`, `as_mut_*`, `map_as_ref_*`, `map_as_mut_*`.
//!
//! All methods here borrow their groups without removing them.
//!
//! **Immutable (`as_ref_*`, `map_as_ref_*`):** require only `G: Borrow<T>`; take `&self`.
//! Work with maps built from immutable slices (`G = &T`) and support concurrent reads.
//! Require the extractor to implement [`SimpleReadFrom`], [`VariantReadFrom`], or [`ReadFrom`].
//!
//! **Mutable (`as_mut_*`, `map_as_mut_*`):** require `G: BorrowMut<T>`; take `&mut self`.
//! Required when mutating fields in-place. Require the extractor to implement
//! [`SimpleExtractFrom`], [`VariantExtractFrom`], or [`ExtractFrom`].
//!
//! Both [`DiscriminantMap`] (closure-based) and [`SplitWithExtractor`] (trait-based and
//! closure-based) implementations live side by side so every family variant is visible
//! at a glance.

use std::mem::Discriminant;
use std::borrow::{Borrow, BorrowMut};

use crate::{DiscriminantMap, Map, Set};
use crate::SplitWithExtractor;
use crate::extractor_traits::{ExtractFrom, SimpleExtractFrom, VariantExtractFrom,
                               ReadFrom, SimpleReadFrom, VariantReadFrom};

// ── DiscriminantMap: closure-based reference access ───────────────────────────

impl<T, G, O> DiscriminantMap<T, G, O> {
    /// Borrow mutable references to inner fields of a single group using an inline closure.
    ///
    /// No extractor struct, no selector type, no turbofish.  `U` is inferred from the
    /// binding type.  See [`SplitWithExtractor::as_mut_simple`] for the trait-based variant.
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
    /// let ints: Vec<&mut i32> = map
    ///     .map_as_mut(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    ///     .unwrap();
    /// ```
    ///
    /// **Lifetime constraint:** `U` must not contain borrows shorter than
    /// `'static`.  For lifetime-carrying outputs use
    /// [`SplitWithExtractor::as_mut_with`].
    ///
    /// # Invariant
    ///
    /// The closure receives `&mut T` and **must not change the discriminant** of
    /// the value it receives.  Writing `*e = DifferentVariant(...)` inside the
    /// closure is valid Rust but leaves the discriminant map in an inconsistent
    /// state: the item will remain in the bucket for `id` despite now having a
    /// different variant.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to the
    /// split function.
    pub fn map_as_mut<'s, U, F>(&'s mut self, id: Discriminant<T>, mut f: F) -> Option<Vec<&'s mut U>>
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

    /// Borrow mutable references to inner fields of multiple groups using an inline closure.
    ///
    /// Batch variant of [`map_as_mut`](Self::map_as_mut).  Returns a map of
    /// extracted mutable references for discriminants that were present.
    ///
    /// # Invariant
    ///
    /// The closure receives `&mut T` and **must not change the discriminant** of
    /// the value it receives.  Writing `*e = DifferentVariant(...)` inside the
    /// closure is valid Rust but leaves the discriminant map in an inconsistent
    /// state: the item will remain in the bucket for `id` despite now having a
    /// different variant.
    pub fn map_as_mut_multiple<'s, U, F>(
        &'s mut self,
        ids: &[Discriminant<T>],
        mut f: F,
    ) -> Map<Discriminant<T>, Vec<&'s mut U>>
    where
        G: BorrowMut<T>,
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        for (id, slice) in self.entries.iter_mut() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter_mut() {
                    if let Some(u) = f(item.borrow_mut()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Borrow immutable references to inner fields of a single group using an inline closure.
    ///
    /// The closure receives `&T` and returns `Option<&U>`.  See
    /// [`map_as_ref_multiple`](Self::map_as_ref_multiple) for the batch variant.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to the
    /// split function.
    pub fn map_as_ref<'s, U, F>(&'s self, id: Discriminant<T>, mut f: F) -> Option<Vec<&'s U>>
    where
        G: Borrow<T>,
        F: for<'a> FnMut(&'a T) -> Option<&'a U>,
    {
        let slice = self.entries.get(&id)?;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter() {
            if let Some(u) = f(item.borrow()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Borrow immutable references to inner fields of multiple groups using an inline closure.
    ///
    /// Batch variant of [`map_as_ref`](Self::map_as_ref).  Returns a map of
    /// extracted immutable references for discriminants that were present.
    pub fn map_as_ref_multiple<'s, U, F>(
        &'s self,
        ids: &[Discriminant<T>],
        mut f: F,
    ) -> Map<Discriminant<T>, Vec<&'s U>>
    where
        G: Borrow<T>,
        F: for<'a> FnMut(&'a T) -> Option<&'a U>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        for (id, slice) in self.entries.iter() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter() {
                    if let Some(u) = f(item.borrow()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Extract owned values from inner fields of a single group using an inline closure,
    /// without removing the group.
    ///
    /// Unlike [`map_as_mut`](Self::map_as_mut) the closure may return an owned `U`
    /// rather than a `&mut U`.  See [`extract_multiple_with`](Self::extract_multiple_with)
    /// for the batch variant, or [`remove_with`](crate::DiscriminantMap::remove_with) for the
    /// consuming variant that removes the group.
    ///
    /// # Invariant
    ///
    /// The closure receives `&mut T` and **must not change the discriminant** of
    /// the value it receives.  Writing `*e = DifferentVariant(...)` inside the
    /// closure is valid Rust but leaves the discriminant map in an inconsistent
    /// state.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to the
    /// split function.
    pub fn extract_with<U, F>(&mut self, id: Discriminant<T>, mut f: F) -> Option<Vec<U>>
    where
        G: BorrowMut<T>,
        F: for<'a> FnMut(&'a mut T) -> Option<U>,
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

    /// Extract owned values from inner fields of multiple groups using an inline closure,
    /// without removing the groups.
    ///
    /// Batch variant of [`extract_with`](Self::extract_with).  Returns a map of
    /// owned extracted values for discriminants that were present.
    /// Unlike [`map_as_mut_multiple`](Self::map_as_mut_multiple) the closure may
    /// return an owned `U` rather than a `&mut U`.
    ///
    /// # Invariant
    ///
    /// The closure receives `&mut T` and **must not change the discriminant** of
    /// the value it receives.  Writing `*e = DifferentVariant(...)` inside the
    /// closure is valid Rust but leaves the discriminant map in an inconsistent
    /// state.
    pub fn extract_multiple_with<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        mut f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        G: BorrowMut<T>,
        F: for<'a> FnMut(&'a mut T) -> Option<U>,
    {
        let mut result = Map::new();
        for id in ids {
            if let Some(slice) = self.entries.get_mut(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter_mut() {
                    if let Some(u) = f(item.borrow_mut()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }
}

// ── SplitWithExtractor: trait-based and closure-based reference access ────────

impl<T, G, O, E> SplitWithExtractor<T, G, O, E>
where
    G: BorrowMut<T>,
{
    /// Extract references using the bound extractor — no binding annotation required.
    ///
    /// The return type is fully determined by `E` and `T`: because
    /// [`SimpleExtractFrom<T>`] declares `Output` as an associated type, the
    /// compiler needs no turbofish.  For the full `'items` lifetime use
    /// [`take_extracted::<()>`](crate::SplitWithExtractor::take_extracted).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom};
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug)] enum E { A(i32), B }
    /// struct EEx;
    /// impl SimpleExtractFrom<E> for EEx {
    ///     type Output = i32;
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
    ///         if let E::A(v) = t { Some(v) } else { None }
    ///     }
    /// }
    ///
    /// let mut data = [E::A(1), E::A(2), E::B];
    /// let a_disc = discriminant(&E::A(0));
    /// let split = split_by_discriminant(&mut data[..], &[a_disc]);
    /// let mut ex = SplitWithExtractor::new(split, EEx);
    /// // No turbofish:
    /// let ints: Vec<&mut i32> = ex.as_mut_simple(a_disc).unwrap();
    /// assert_eq!(ints.len(), 2);
    /// ```
    pub fn as_mut_simple(&mut self, id: Discriminant<T>) -> Option<Vec<&mut <E as SimpleExtractFrom<T>>::Output>>
    where
        E: SimpleExtractFrom<T>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice {
            if let Some(u) = <E as SimpleExtractFrom<T>>::extract_from(extractor, item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract field references from a group — `U` inferred from the binding type, no turbofish.
    ///
    /// Each call resolves its own independent `U` from the binding annotation.
    /// The extractor must implement [`VariantExtractFrom<T, U>`] for each `U` it handles.
    ///
    /// - For a single-variant extractor where no annotation at all is desired, use
    ///   [`as_mut_simple`](Self::as_mut_simple) instead.
    /// - For multi-field outputs or multiple selectors on the same `(E, T)` pair, use
    ///   [`as_mut_with<S>`](Self::as_mut_with) with an explicit selector ZST.
    /// - For the consuming counterpart that preserves the full `'items` lifetime, see
    ///   [`take_extracted`](crate::SplitWithExtractor::take_extracted).
    /// - For the batch variant, see [`as_mut_multiple`](Self::as_mut_multiple).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::{VariantExtractFrom, split_by_discriminant, SplitWithExtractor};
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
    /// struct EEx;
    /// impl VariantExtractFrom<E, i32> for EEx {
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
    ///         if let E::A(v) = t { Some(v) } else { None }
    ///     }
    /// }
    /// impl VariantExtractFrom<E, String> for EEx {
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut String> {
    ///         if let E::B(s) = t { Some(s) } else { None }
    ///     }
    /// }
    ///
    /// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    /// let mut ex = SplitWithExtractor::new(split, EEx);
    ///
    /// let ints: Vec<&mut i32>    = ex.as_mut(a_disc).unwrap();
    /// drop(ints);
    /// let strs: Vec<&mut String> = ex.as_mut(b_disc).unwrap();
    /// assert_eq!(strs.len(), 1);
    /// ```
    pub fn as_mut<U>(&mut self, id: Discriminant<T>) -> Option<Vec<&mut U>>
    where
        E: VariantExtractFrom<T, U>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice {
            if let Some(u) = <E as VariantExtractFrom<T, U>>::extract_from(extractor, item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract references using the bound extractor with an explicit selector type.
    ///
    /// Unlike [`as_mut`](Self::as_mut), this accepts any [`ExtractFrom<T, S>`] impl —
    /// including multi-field tuple outputs and multiple selectors on the same `(E, T)` pair.
    /// The selector `S` must be named at the call site: `ex.as_mut_with::<MySelector>(disc)`.
    ///
    /// - For the `SimpleExtractFrom` (zero-annotation) variant, see [`as_mut_simple`](Self::as_mut_simple).
    /// - For single-type variants where `U` is inferred from the binding, see [`as_mut`](Self::as_mut).
    /// - For the consuming counterpart that preserves the full `'items` lifetime, see
    ///   [`take_extracted<S>`](crate::SplitWithExtractor::take_extracted).
    /// - For the batch variant, see [`as_mut_multiple_with`](Self::as_mut_multiple_with).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug)] enum E { A(i32), B(String) }
    /// struct EEx;
    /// pub struct SelectA;
    /// pub struct SelectB;
    ///
    /// impl ExtractFrom<E, SelectA> for EEx {
    ///     type Output<'a> = &'a mut i32;
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<Self::Output<'a>> {
    ///         if let E::A(v) = t { Some(v) } else { None }
    ///     }
    /// }
    /// impl ExtractFrom<E, SelectB> for EEx {
    ///     type Output<'a> = &'a mut String;
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<Self::Output<'a>> {
    ///         if let E::B(s) = t { Some(s) } else { None }
    ///     }
    /// }
    ///
    /// let mut data = [E::A(1), E::A(2), E::B("hi".into())];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    /// let mut ex = SplitWithExtractor::new(split, EEx);
    ///
    /// { let ints: Vec<&mut i32>    = ex.as_mut_with::<SelectA>(a_disc).unwrap(); assert_eq!(ints.len(), 2); }
    /// { let strs: Vec<&mut String> = ex.as_mut_with::<SelectB>(b_disc).unwrap(); assert_eq!(strs.len(), 1); }
    /// ```
    pub fn as_mut_with<'s, S>(&'s mut self, id: Discriminant<T>) -> Option<Vec<<E as ExtractFrom<T, S>>::Output<'s>>>
    where
        E: ExtractFrom<T, S>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice {
            if let Some(u) = extractor.extract_from(item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract field references via an inline closure — no extractor struct, no turbofish.
    ///
    /// Delegates to [`DiscriminantMap::map_as_mut`] on the inner map.
    /// Use this when the bound extractor `E` cannot cover the variant or when a
    /// one-off closure is cleaner than a full extractor impl.
    ///
    /// - For trait-based extraction without a closure, see [`as_mut_simple`](Self::as_mut_simple),
    ///   [`as_mut<U>`](Self::as_mut), or [`as_mut_with<S>`](Self::as_mut_with).
    /// - For the immutable counterpart, see [`map_as_ref`](Self::map_as_ref).
    /// - For the batch variant, see [`map_as_mut_multiple`](Self::map_as_mut_multiple).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    pub fn map_as_mut<'s, U, F>(&'s mut self, id: Discriminant<T>, f: F) -> Option<Vec<&'s mut U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        self.inner.map_as_mut(id, f)
    }

    /// Batch mutable access — `U` inferred from the binding annotation.
    ///
    /// Batch variant of [`as_mut`](Self::as_mut).  Returns a map of mutable references
    /// for all requested discriminants that were present.
    pub fn as_mut_multiple<U>(
        &mut self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<&mut U>>
    where
        E: VariantExtractFrom<T, U>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter_mut() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter_mut() {
                    if let Some(u) = <E as VariantExtractFrom<T, U>>::extract_from(extractor, item.borrow_mut()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Batch mutable access using the bound `SimpleExtractFrom` extractor — no annotation required.
    ///
    /// Batch variant of [`as_mut_simple`](Self::as_mut_simple).  Returns a map of
    /// mutable references for all requested discriminants that were present.
    pub fn as_mut_multiple_simple(
        &mut self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<&mut <E as SimpleExtractFrom<T>>::Output>>
    where
        E: SimpleExtractFrom<T>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter_mut() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter_mut() {
                    if let Some(u) = <E as SimpleExtractFrom<T>>::extract_from(extractor, item.borrow_mut()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Batch mutable access using the selector — batch variant of [`as_mut_with`](Self::as_mut_with).
    ///
    /// Returns a map of mutable references for all requested discriminants that were present.
    pub fn as_mut_multiple_with<'s, S>(
        &'s mut self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<<E as ExtractFrom<T, S>>::Output<'s>>>
    where
        E: ExtractFrom<T, S>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter_mut() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter_mut() {
                    if let Some(u) = extractor.extract_from(item.borrow_mut()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Batch mutable access via an inline closure — delegates to
    /// [`DiscriminantMap::map_as_mut_multiple`].
    ///
    /// For the single-discriminant variant, see [`map_as_mut`](Self::map_as_mut).
    pub fn map_as_mut_multiple<'s, U, F>(
        &'s mut self,
        ids: &[Discriminant<T>],
        f: F,
    ) -> Map<Discriminant<T>, Vec<&'s mut U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        self.inner.map_as_mut_multiple(ids, f)
    }

    /// Extract owned values from a single group using an inline closure, without removing it.
    ///
    /// Delegates to [`DiscriminantMap::extract_with`].  The closure may return any owned
    /// `U` rather than `&mut U`.  See [`extract_multiple_with`](Self::extract_multiple_with)
    /// for the batch variant.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    pub fn extract_with<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<U>,
    {
        self.inner.extract_with(id, f)
    }

    /// Extract owned values from multiple groups using an inline closure, without removing them.
    ///
    /// Delegates to [`DiscriminantMap::extract_multiple_with`].  Returns a map of
    /// owned extracted values for discriminants that were present.
    pub fn extract_multiple_with<U, F>(
        &mut self,
        ids: &[Discriminant<T>],
        f: F,
    ) -> Map<Discriminant<T>, Vec<U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<U>,
    {
        self.inner.extract_multiple_with(ids, f)
    }
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E>
where
    G: Borrow<T>,
{
    /// Extract field references via an inline closure — immutable borrow, no turbofish.
    ///
    /// The closure receives `&T` and returns `Option<&U>`.  To extract from
    /// multiple groups, call with different discriminants.
    ///
    /// - For the mutable counterpart (modifies in-place), see [`map_as_mut`](Self::map_as_mut).
    /// - For the batch variant, see [`map_as_ref_multiple`](Self::map_as_ref_multiple).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
    ///
    /// let data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    /// let ex = SplitWithExtractor::new(split, ());
    ///
    /// let ints: Vec<&i32> = ex
    ///     .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    ///     .unwrap();
    /// let strs: Vec<&String> = ex
    ///     .map_as_ref(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
    ///     .unwrap();
    /// ```
    pub fn map_as_ref<'s, U, F>(&'s self, id: Discriminant<T>, f: F) -> Option<Vec<&'s U>>
    where
        F: for<'a> FnMut(&'a T) -> Option<&'a U>,
    {
        self.inner.map_as_ref(id, f)
    }

    /// Batch immutable access via an inline closure — delegates to
    /// [`DiscriminantMap::map_as_ref_multiple`].
    ///
    /// For the single-discriminant variant, see [`map_as_ref`](Self::map_as_ref).
    pub fn map_as_ref_multiple<'s, U, F>(
        &'s self,
        ids: &[Discriminant<T>],
        f: F,
    ) -> Map<Discriminant<T>, Vec<&'s U>>
    where
        F: for<'a> FnMut(&'a T) -> Option<&'a U>,
    {
        self.inner.map_as_ref_multiple(ids, f)
    }

    // ── Trait-based immutable reference access ──────────────────────────────────

    /// Borrow immutable references using the bound extractor — no annotation needed.
    ///
    /// The return type is fully determined by `E` and `T` via the
    /// [`SimpleReadFrom`] associated type.  Takes `&self` (no mutable borrow
    /// required).  Works even when the map was built from an immutable slice
    /// (`G = &T`) because only [`Borrow<T>`][std::borrow::Borrow] is required.
    ///
    /// - For the mutable counterpart see [`as_mut_simple`](Self::as_mut_simple).
    /// - For variant inference via binding type see [`as_ref`](Self::as_ref).
    /// - For multi-field or selector-based outputs see [`as_ref_with`](Self::as_ref_with).
    /// - For the batch variant see [`as_ref_multiple_simple`](Self::as_ref_multiple_simple).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    pub fn as_ref_simple<'s>(
        &'s self,
        id: Discriminant<T>,
    ) -> Option<Vec<&'s <E as SimpleReadFrom<T>>::Output>>
    where
        E: SimpleReadFrom<T>,
    {
        let slice = self.inner.get(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter() {
            if let Some(u) = <E as SimpleReadFrom<T>>::read_from(extractor, item.borrow()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Borrow immutable field references — `U` inferred from the binding, no turbofish.
    ///
    /// The extractor must implement [`VariantReadFrom<T, U>`] for each `U` it
    /// handles.  Because `SimpleReadFrom<T>` automatically blankets
    /// `VariantReadFrom<T, Output>`, the primary output type is available for
    /// free; additional `U` types require a separate `VariantReadFrom` impl.
    ///
    /// - For the zero-annotation variant see [`as_ref_simple`](Self::as_ref_simple).
    /// - For multi-field or GAT outputs see [`as_ref_with`](Self::as_ref_with).
    /// - For the batch variant see [`as_ref_multiple`](Self::as_ref_multiple).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    pub fn as_ref<'s, U>(
        &'s self,
        id: Discriminant<T>,
    ) -> Option<Vec<&'s U>>
    where
        E: VariantReadFrom<T, U>,
    {
        let slice = self.inner.get(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter() {
            if let Some(u) = <E as VariantReadFrom<T, U>>::read_from(extractor, item.borrow()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Borrow immutable references using an explicit selector type.
    ///
    /// Like [`as_mut_with`](Self::as_mut_with) but immutable.  The selector `S`
    /// must be named at the call site: `ex.as_ref_with::<MySelector>(disc)`.
    /// Useful for multi-field tuple outputs and multiple selectors on the same
    /// `(E, T)` pair.
    ///
    /// - For the zero-annotation variant see [`as_ref_simple`](Self::as_ref_simple).
    /// - For binding-inferred `U` see [`as_ref`](Self::as_ref).
    /// - For the batch variant see [`as_ref_multiple_with`](Self::as_ref_multiple_with).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`split_by_discriminant`](crate::split_by_discriminant).
    pub fn as_ref_with<'s, S>(
        &'s self,
        id: Discriminant<T>,
    ) -> Option<Vec<<E as ReadFrom<T, S>>::Output<'s>>>
    where
        E: ReadFrom<T, S>,
    {
        let slice = self.inner.get(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter() {
            if let Some(u) = <E as ReadFrom<T, S>>::read_from(extractor, item.borrow()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Batch immutable access using the bound `SimpleReadFrom` extractor.
    ///
    /// Batch variant of [`as_ref_simple`](Self::as_ref_simple).  Returns a map
    /// of immutable references for all requested discriminants that were present.
    pub fn as_ref_multiple_simple<'s>(
        &'s self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<&'s <E as SimpleReadFrom<T>>::Output>>
    where
        E: SimpleReadFrom<T>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter() {
                    if let Some(u) = <E as SimpleReadFrom<T>>::read_from(extractor, item.borrow()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Batch immutable references — `U` inferred from the binding annotation.
    ///
    /// Batch variant of [`as_ref`](Self::as_ref).  Returns a map of immutable
    /// references for all requested discriminants that were present.
    pub fn as_ref_multiple<'s, U>(
        &'s self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<&'s U>>
    where
        E: VariantReadFrom<T, U>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter() {
                    if let Some(u) = <E as VariantReadFrom<T, U>>::read_from(extractor, item.borrow()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }

    /// Batch immutable references with selector — batch variant of [`as_ref_with`](Self::as_ref_with).
    ///
    /// Returns a map of immutable references for all requested discriminants that were present.
    pub fn as_ref_multiple_with<'s, S>(
        &'s self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<<E as ReadFrom<T, S>>::Output<'s>>>
    where
        E: ReadFrom<T, S>,
    {
        let wanted: Set<Discriminant<T>> = ids.iter().copied().collect();
        let mut result = Map::new();
        let extractor = &self.extractor;
        for (id, slice) in self.inner.entries.iter() {
            if wanted.contains(id) {
                let mut out = Vec::with_capacity(slice.len());
                for item in slice.iter() {
                    if let Some(u) = <E as ReadFrom<T, S>>::read_from(extractor, item.borrow()) {
                        out.push(u);
                    }
                }
                result.insert(*id, out);
            }
        }
        result
    }
}
