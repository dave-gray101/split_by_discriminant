use std::borrow::BorrowMut;
use std::mem::Discriminant;

use crate::DiscriminantMap;

use crate::extractor_traits::{ExtractFrom, SimpleExtractFrom, SplitWithExtractor, TakeFrom, VariantExtractFrom};

/// Blanket: SimpleExtractFrom<T> → ExtractFrom<T, ()>
///
/// Coherent because `Output` is an associated type — every (E, T) pair maps to
/// exactly one Output, so at most one `ExtractFrom<T, ()>` is generated here.
/// Do not implement `ExtractFrom<T, ()>` manually alongside `SimpleExtractFrom<T>`.
impl<E, T> ExtractFrom<T, ()> for E
where
    E: SimpleExtractFrom<T>,
{
    type Output<'a>
        = &'a mut <E as SimpleExtractFrom<T>>::Output
    where
        T: 'a;
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<Self::Output<'a>> {
        <E as SimpleExtractFrom<T>>::extract_from(self, t)
    }
}

/// Blanket: SimpleExtractFrom<T> → VariantExtractFrom<T, Output>
///
/// An extractor that implements SimpleExtractFrom<T> automatically gets the
/// VariantExtractFrom<T, Output> impl for free, so SplitWithExtractor::extract()
/// works on its associated variant without any additional impl.
/// Do not manually implement VariantExtractFrom<T, U> when U == SimpleExtractFrom<T>::Output.
impl<E, T> VariantExtractFrom<T, <E as SimpleExtractFrom<T>>::Output> for E
where
    E: SimpleExtractFrom<T>,
{
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut <E as SimpleExtractFrom<T>>::Output> {
        <E as SimpleExtractFrom<T>>::extract_from(self, t)
    }
}

/// Blanket: every ExtractFrom<T, S> is automatically a TakeFrom<&'a mut T, S>.
impl<'a, T, S, E> TakeFrom<&'a mut T, S> for E
where
    E: ExtractFrom<T, S>,
{
    type Output = E::Output<'a>;
    fn take_from(&self, g: &'a mut T) -> Option<Self::Output> {
        self.extract_from(g)
    }
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E> {
    /// Wrap a split and an extractor together.
    pub fn new(split: DiscriminantMap<T, G, O>, extractor: E) -> Self {
        SplitWithExtractor { inner: split, extractor }
    }

    /// Unwrap back to the underlying [`DiscriminantMap`].
    pub fn into_inner(self) -> DiscriminantMap<T, G, O> {
        self.inner
    }

    /// Borrow the unmatched items.
    pub fn others(&self) -> &[O] {
        self.inner.others()
    }

    /// Access the stored group for a discriminant as a shared slice.
    pub fn get(&self, id: Discriminant<T>) -> Option<&[G]> {
        self.inner.get(id)
    }

    /// Mutably borrow the group for `id` as a mutable slice.
    pub fn get_mut(&mut self, id: Discriminant<T>) -> Option<&mut [G]> {
        self.inner.get_mut(id)
    }

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

    /// Consume and extract the inner values of the [`SimpleExtractFrom`] variant —
    /// no turbofish and no annotation required at the call site.
    ///
    /// This is the consuming counterpart of [`extract_simple`](Self::extract_simple).
    /// It closes the ergonomic gap left by [`take_extracted`](Self::take_extracted):
    /// the `<()>` turbofish is absent, and the return type is fully determined by
    /// `E` and `T` exactly as with `extract_simple`.
    ///
    /// Returned elements carry the full original `'items` lifetime and can outlive
    /// the `SplitWithExtractor`.
    ///
    /// When `G = &mut T` the `E: TakeFrom<G, ()>` bound is satisfied automatically
    /// via the `SimpleExtractFrom → ExtractFrom<T,()> → TakeFrom` blanket chain;
    /// no extra impl is needed.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`crate::split_by_discriminant`], or if the group has already been removed.
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
    /// // ints outlives the SplitWithExtractor — no turbofish, no annotation:
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

    /// Remove the group for `id` and extract inner values using the bound
    /// extractor — no closure needed.
    ///
    /// The bound `E: TakeFrom<G, S>` is satisfied automatically for the common
    /// `G = &mut T` case via the blanket impl over `ExtractFrom<T, S>` — you
    /// do not need to implement `TakeFrom` separately.
    ///
    /// `S` is the selector type that disambiguates which `TakeFrom` impl to use.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`crate::split_by_discriminant`], or if the group has already been removed.
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
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E>
where
    G: BorrowMut<T>,
{
    /// Extract references using the bound extractor — no binding annotation required.
    ///
    /// The return type is fully determined by `E` and `T`: because
    /// [`SimpleExtractFrom<T>`] declares `Output` as an associated type, there
    /// is at most one `&mut U` per `(E, T)` pair, so the compiler needs no
    /// turbofish and no binding annotation at all.  For the full `'items`
    /// lifetime use [`take_extracted::<()>`](Self::take_extracted).
    ///
    /// When your extractor covers multiple variants, use [`extract`](Self::extract)
    /// instead — it requires a type annotation on the binding but supports any
    /// number of `VariantExtractFrom<T, U>` impls on the same extractor.
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`crate::split_by_discriminant`].
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
    /// let ints: Vec<&mut i32> = ex.extract_simple(a_disc).unwrap();
    /// assert_eq!(ints.len(), 2);
    /// ```
    pub fn extract_simple(&mut self, id: Discriminant<T>) -> Option<Vec<&mut <E as SimpleExtractFrom<T>>::Output>>
    where
        E: SimpleExtractFrom<T>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter_mut() {
            if let Some(u) = <E as SimpleExtractFrom<T>>::extract_from(extractor, item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract field references from a group — `U` inferred from the binding type, no turbofish.
    ///
    /// This is the primary extraction method.  Each call resolves its own independent `U`
    /// from the binding annotation, so every simple-field variant of an enum is reachable
    /// without any selector types or turbofish syntax:
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
    /// let ints: Vec<&mut i32>    = ex.extract(a_disc).unwrap();
    /// assert_eq!(ints.len(), 2);
    /// drop(ints); // end the borrow before the next call
    /// let strs: Vec<&mut String> = ex.extract(b_disc).unwrap();
    /// assert_eq!(strs.len(), 1);
    /// ```
    ///
    /// The extractor must implement [`VariantExtractFrom<T, U>`] for each `U` it handles.
    /// If it already implements [`SimpleExtractFrom<T>`], the blanket provides
    /// `VariantExtractFrom<T, Output>` for free — no additional impl needed for that variant.
    ///
    /// For multi-field tuple/struct outputs use [`extract_gat::<S>`](Self::extract_gat).
    /// For fully-annotation-free extraction (no binding type needed) use
    /// [`extract_simple`](Self::extract_simple).
    pub fn extract<U>(&mut self, id: Discriminant<T>) -> Option<Vec<&mut U>>
    where
        E: VariantExtractFrom<T, U>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter_mut() {
            if let Some(u) = <E as VariantExtractFrom<T, U>>::extract_from(extractor, item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract references using the bound extractor with an explicit selector type.
    ///
    /// Unlike [`extract`](Self::extract), this method accepts any
    /// [`ExtractFrom<T, S>`] impl — including multi-field tuple outputs and
    /// multiple selectors on the same `(E, T)` pair.  The selector `S` must be
    /// named at the call site: `ex.extract_gat::<MySelector>(disc)`.
    ///
    /// Reborrows each group element through `&mut self`, so the returned
    /// references' lifetime is tied to this call.  For the full `'items`
    /// lifetime, use [`take_extracted`](Self::take_extracted).
    ///
    /// Returns `None` if `id` was not among the discriminants passed to
    /// [`crate::split_by_discriminant`].
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
    ///
    /// let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    /// let mut ex = SplitWithExtractor::new(split, EEx);
    ///
    /// { let ints: Vec<&mut i32>    = ex.extract_gat::<SelectA>(a_disc).unwrap(); assert_eq!(ints.len(), 2); }
    /// { let strs: Vec<&mut String> = ex.extract_gat::<SelectB>(b_disc).unwrap(); assert_eq!(strs.len(), 1); }
    /// ```
    pub fn extract_gat<'s, S>(&'s mut self, id: Discriminant<T>) -> Option<Vec<<E as ExtractFrom<T, S>>::Output<'s>>>
    where
        E: ExtractFrom<T, S>,
    {
        let slice = self.inner.get_mut(id)?;
        let extractor = &self.extractor;
        let mut out = Vec::with_capacity(slice.len());
        for item in slice.iter_mut() {
            if let Some(u) = extractor.extract_from(item.borrow_mut()) {
                out.push(u);
            }
        }
        Some(out)
    }

    /// Extract field references via an inline closure — no extractor struct,
    /// no selector type, no turbofish.
    ///
    /// Delegates to [`DiscriminantMap::extract_with`] on the inner map.
    /// The bound extractor `E` is not used; the closure supplies the extraction
    /// logic directly.  Use this when you need turbofish-free access to a
    /// variant whose field type cannot be made the sole `Output` of the
    /// bound extractor.
    ///
    /// `U` is inferred from the expected binding type:
    ///
    /// ```rust
    /// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom};
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
    /// struct Ex;
    /// impl SimpleExtractFrom<E> for Ex {
    ///     type Output = i32;
    ///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
    ///         if let E::A(v) = t { Some(v) } else { None }
    ///     }
    /// }
    ///
    /// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    /// let a_disc = discriminant(&E::A(0));
    /// let b_disc = discriminant(&E::B(String::new()));
    /// let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    /// let mut ex = SplitWithExtractor::new(split, Ex);
    ///
    /// // Both variants — no turbofish on either:
    /// let ints: Vec<&mut i32> = ex
    ///     .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    ///     .unwrap();
    /// let strs: Vec<&mut String> = ex
    ///     .extract_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
    ///     .unwrap();
    /// ```
    pub fn extract_with<'s, U, F>(&'s mut self, id: Discriminant<T>, f: F) -> Option<Vec<&'s mut U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        self.inner.extract_with(id, f)
    }
}

// ── IntoIterator delegation ───────────────────────────────────────────────────

impl<T, G, O, E> IntoIterator for SplitWithExtractor<T, G, O, E> {
    type Item = (Discriminant<T>, Vec<G>);
    type IntoIter = <DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T, G, O, E> IntoIterator for &'a SplitWithExtractor<T, G, O, E> {
    type Item = (&'a Discriminant<T>, &'a [G]);
    type IntoIter = <&'a DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

impl<'a, T, G, O, E> IntoIterator for &'a mut SplitWithExtractor<T, G, O, E> {
    type Item = (&'a Discriminant<T>, &'a mut [G]);
    type IntoIter = <&'a mut DiscriminantMap<T, G, O> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.inner).into_iter()
    }
}
