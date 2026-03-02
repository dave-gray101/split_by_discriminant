use std::mem::Discriminant;
use std::borrow::BorrowMut;

use crate::SplitByDiscriminant;

/// Defines how to obtain a `&mut U` from a `&mut T`.
///
/// Unlike implementing a trait directly on `T`, the impl here lives on a
/// *local extractor type* that you define in your own crate.  This means the
/// orphan rule is never an issue, even when both `T` and `U` come from
/// external crates.
///
/// # Example — foreign enum
///
/// ```rust
/// use split_by_discriminant::ExtractFrom;
///
/// // In a real project this enum would live in an external crate.
/// // The key point: the impl below is on *MyEnumExtractor*, which is local,
/// // so the orphan rule is never triggered regardless of where MyEnum lives.
/// #[derive(Debug)] enum MyEnum { A(i32), B }
///
/// pub struct MyEnumExtractor;
///
/// impl ExtractFrom<MyEnum, i32> for MyEnumExtractor {
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<&'a mut i32> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
/// ```
///
/// Pass `MyEnumExtractor` to [`SplitWithExtractor::new`] and call
/// [`SplitWithExtractor::extract`] with no closure at the call site.
pub trait ExtractFrom<T, U> {
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut U>;
}

/// A [`SplitByDiscriminant`] with an extractor bound up front.
///
/// This wrapper carries **four** generic parameters:
///
/// * `T` – same as on the inner split, the type whose discriminant keys the
///   groups.
/// * `G` – element type stored in each group; forwarded from the inner
///   `SplitByDiscriminant`.
/// * `O` – element type for the others bucket; also forwarded from the inner
///   split and defaults to `G` when constructing the split itself.
/// * `E` – an *extractor value* that implements `ExtractFrom<T, U>` for one or
///   more output types `U`.  This parameter is what makes the API
///   ergonomic: once you create `SplitWithExtractor<T, G, O, E>`, calls to
///   `extract::<U>(id)` automatically select the correct `U` based on the
///   `E: ExtractFrom<T, U>` impl in scope.  The extractor type is typically a
///   zero‑sized struct defined locally in your crate.
///
/// The struct simply holds the inner split and the extractor.  You can
/// unwrap it with `into_inner` when you need consuming helpers.
///
/// Construct with [`SplitWithExtractor::new`] after calling
/// [`split_by_discriminant`].  Non-consuming methods
/// ([`group`](SplitWithExtractor::group), [`extract_with`](SplitWithExtractor::extract_with),
/// [`extract`](SplitWithExtractor::extract)) are available directly on
/// `SplitWithExtractor`.  To reach consuming methods (`into_parts`,
/// `map_groups`, `map_others`), call [`into_inner`](SplitWithExtractor::into_inner)
/// first.
///
/// # Example
///
/// The code below is entirely self-contained to a single module due to doctest limitations.
/// Comments mark what would be a separate crate in a real project.
///
/// ```rust
/// // ── external_enums ──────────────────────────────────────────────────────
/// // Foreign crate: we cannot change it or derive anything on it.
/// #[derive(Debug)] pub enum MyEnum { A(i32), B(String) }
///
/// // ── user_helper ─────────────────────────────────────────────────────────
/// // Glue crate: depends on split_by_discriminant + external_enums.
/// // MyEnumExtractor is LOCAL to this crate, so every ExtractFrom impl is
/// // orphan-rule–safe regardless of where MyEnum was defined.
/// use split_by_discriminant::ExtractFrom;
/// struct MyEnumExtractor;
/// impl ExtractFrom<MyEnum, i32> for MyEnumExtractor {
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<&'a mut i32> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
/// impl ExtractFrom<MyEnum, String> for MyEnumExtractor {
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<&'a mut String> {
///         if let MyEnum::B(s) = t { Some(s) } else { None }
///     }
/// }
///
/// // ── user_downstream ─────────────────────────────────────────────────────
/// // Calls SplitWithExtractor::extract with no closure needed at the call site.
/// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
/// use std::mem::discriminant;
///
/// let mut data = vec![MyEnum::A(1), MyEnum::B("hi".into()), MyEnum::A(2)];
/// let a_disc = discriminant(&MyEnum::A(0));
/// let b_disc = discriminant(&MyEnum::B(String::new()));
///
/// let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
/// let mut extractor = SplitWithExtractor::new(split, MyEnumExtractor);
///
/// // Each extract call lives in its own scope so the &mut borrows don't overlap.
/// { let v: Vec<&mut i32>    = extractor.extract(a_disc).unwrap(); assert_eq!(v.len(), 2); }
/// { let v: Vec<&mut String> = extractor.extract(b_disc).unwrap(); assert_eq!(v.len(), 1); }
///
/// // Consuming helpers reached via into_inner().
/// let (_groups, others) = extractor.into_inner().into_parts();
/// assert_eq!(others.len(), 0);
/// ```
///
/// A [`SplitByDiscriminant`] paired with a user‑supplied extractor value.
///
/// This wrapper carries exactly the same `T`, `G`, and `O` parameters as the
/// inner split (they describe the enum type and the element types for groups
/// and others).  The extra parameter `E` is the extractor type that implements `ExtractFrom<T, U>` for one or more output types `U`; it allows the
/// ergonomic `extract` method to infer `U` without a closure at the call
/// site.  Because the impl lives on a *local* extractor type, the orphan rule
/// is satisfied even when `T` and `U` are foreign.
///
/// Type parameters:
/// * `T` – the enum/`Discriminant` target for the split.
/// * `G` – element type stored in each matching group.
/// * `O` – element type stored in `others` (defaults to `G`).
/// * `E` – extractor value type implementing `ExtractFrom<T, U>`.
///
/// See the crate documentation and README for examples and the four‑crate
/// pattern that motivates this design.
pub struct SplitWithExtractor<T, G, O, E> {
    inner: SplitByDiscriminant<T, G, O>,
    extractor: E,
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E> {
    /// Wrap a split and an extractor together.
    pub fn new(split: SplitByDiscriminant<T, G, O>, extractor: E) -> Self {
        SplitWithExtractor { inner: split, extractor }
    }

    /// Unwrap back to the underlying [`SplitByDiscriminant`].
    ///
    /// Use this to reach consuming methods (`into_parts`, `map_groups`,
    /// `map_others`) on the inner split.
    pub fn into_inner(self) -> SplitByDiscriminant<T, G, O> {
        self.inner
    }

    /// Access the stored group for a discriminant.
    pub fn group(&mut self, id: Discriminant<T>) -> Option<&Vec<G>> {
        self.inner.group(id)
    }

    /// Remove and return the owned group for `id`, forwarded to the inner
    /// [`SplitByDiscriminant`].
    ///
    /// See [`SplitByDiscriminant::take_group`] for full documentation on
    /// the lifetime-preservation guarantee and idiomatic usage.
    pub fn take_group(&mut self, id: Discriminant<T>) -> Option<Vec<G>> {
        self.inner.take_group(id)
    }
}

impl<T, G, O, E> SplitWithExtractor<T, G, O, E>
where
    G: BorrowMut<T>,
{
    /// Closure-based extraction, forwarded to the inner split.
    ///
    /// See [`SplitByDiscriminant::extract_with`] for full documentation.
    pub fn extract_with<U, F>(&mut self, id: Discriminant<T>, f: F) -> Option<Vec<&mut U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        self.inner.extract_with(id, f)
    }

    /// Extract inner values using the bound extractor — no closure needed.
    ///
    /// `U` is inferred from the `E: ExtractFrom<T, U>` bound and the
    /// call-site type annotation.  Returns `None` if `id` was not among the
    /// discriminants passed to [`split_by_discriminant`].
    pub fn extract<U>(&mut self, id: Discriminant<T>) -> Option<Vec<&mut U>>
    where
        E: ExtractFrom<T, U>,
    {
        let extractor = &self.extractor;
        self.inner.extract_with(id, |t| extractor.extract_from(t))
    }
}
