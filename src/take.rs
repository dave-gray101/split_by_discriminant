//! Consuming extraction via extractor traits: `take_*` methods on [`SplitWithExtractor`].
//!
//! All methods here remove groups and return owned values extracted via
//! [`TakeFrom`](crate::TakeFrom) / [`SimpleExtractFrom`](crate::SimpleExtractFrom).
//! For non-removing access use the methods in [`ref_access`](crate::ref_access).

use std::mem::Discriminant;

use crate::{Map, SplitWithExtractor};
use crate::extractor_traits::{SimpleExtractFrom, TakeFrom};

impl<T, G, O, E> SplitWithExtractor<T, G, O, E> {
    /// Consume and extract the inner values of the [`SimpleExtractFrom`] variant —
    /// no turbofish and no annotation required at the call site.
    ///
    /// This is the consuming counterpart of [`as_mut_simple`](crate::SplitWithExtractor::as_mut_simple).
    /// Returned elements carry the full original `'items` lifetime and can outlive
    /// the `SplitWithExtractor`.
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
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B }
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
    ///
    /// let mut ints = {
    ///     let split = split_by_discriminant(&mut data[..], &[a_disc]);
    ///     let mut ex = SplitWithExtractor::new(split, EEx);
    ///     ex.take_simple(a_disc).unwrap()
    /// };
    /// assert_eq!(ints.len(), 2);
    /// let first = &mut ints[0];
    /// **first = 99;
    /// drop(ints);
    /// assert_eq!(data[0], E::A(99));
    /// ```
    pub fn take_simple(&mut self, id: Discriminant<T>) -> Option<Vec<<E as TakeFrom<G, ()>>::Output>>
    where
        E: SimpleExtractFrom<T>,
        E: TakeFrom<G, ()>,
    {
        self.take_extracted::<()>(id)
    }

    /// Remove the group for `id` and extract inner values using the bound extractor.
    ///
    /// The bound `E: TakeFrom<G, S>` is satisfied automatically for the common
    /// `G = &mut T` case via the blanket impl over `ExtractFrom<T, S>`.
    /// `S` is the selector type that disambiguates which `TakeFrom` impl to use.
    ///
    /// - For the `SimpleExtractFrom` (zero-annotation) variant, see [`take_simple`](Self::take_simple).
    /// - For the non-consuming reborrow counterpart, see [`as_mut_with<S>`](crate::SplitWithExtractor::as_mut_with).
    /// - For the batch variant, see [`take_multiple_extracted`](Self::take_multiple_extracted).
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
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B }
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
    ///
    /// let mut ints: Vec<&mut i32> = {
    ///     let split = split_by_discriminant(&mut data[..], &[a_disc]);
    ///     let mut ex = SplitWithExtractor::new(split, EEx);
    ///     ex.take_extracted::<()>(a_disc).unwrap()
    /// };
    /// assert_eq!(ints.len(), 2);
    /// let first = &mut ints[0];
    /// **first = 99;
    /// drop(ints);
    /// assert_eq!(data[0], E::A(99));
    /// ```
    pub fn take_extracted<S>(&mut self, id: Discriminant<T>) -> Option<Vec<<E as TakeFrom<G, S>>::Output>>
    where
        E: TakeFrom<G, S>,
    {
        let extractor = &self.extractor;
        self.inner.remove_with(id, |g| extractor.take_from(g))
    }

    /// Extract from inner fields of multiple groups and remove them — using the bound
    /// `SimpleExtractFrom` extractor with no turbofish or annotation.
    ///
    /// Batch variant of [`take_simple`](Self::take_simple).  Returns a map of
    /// extracted values for discriminants that were present.
    pub fn take_multiple_simple(&mut self, ids: &[Discriminant<T>]) -> Map<Discriminant<T>, Vec<<E as TakeFrom<G, ()>>::Output>>
    where
        E: SimpleExtractFrom<T>,
        E: TakeFrom<G, ()>,
    {
        self.take_multiple_extracted::<()>(ids)
    }

    /// Extract from inner fields of multiple groups and remove them — using the bound
    /// extractor with an explicit selector type.
    ///
    /// Batch variant of [`take_extracted`](Self::take_extracted).  Returns a map of
    /// extracted values for discriminants that were present.
    pub fn take_multiple_extracted<S>(
        &mut self,
        ids: &[Discriminant<T>],
    ) -> Map<Discriminant<T>, Vec<<E as TakeFrom<G, S>>::Output>>
    where
        E: TakeFrom<G, S>,
    {
        let extractor = &self.extractor;
        self.inner.remove_multiple_with(ids, |g| extractor.take_from(g))
    }
}
