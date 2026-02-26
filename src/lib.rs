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

use std::mem::Discriminant;
use std::borrow::{Borrow, BorrowMut};

/// Outcome of a discriminant‑based partition operation.
///
/// `SplitByDiscriminant` is generic over the **reference type** `R` produced by
/// the input iterator, so it can represent either mutable or immutable
/// borrows or owned values.  The only requirement on `R` is that it implement
/// [`Borrow<T>`]; this allows the implementation to obtain a `&T` for
/// discriminant computation.  Note that `Borrow<T>` is blanket‑implemented for
/// `T` itself, so `R` may be `T` when you are consuming owned items.
///
/// A lifetime parameter ties the returned references to the original
/// collection when `R` is a reference.  The caller can also write the type
/// explicitly (`SplitByDiscriminant<T, &T>` or `SplitByDiscriminant<T, &mut T>`),
/// but type inference usually makes that unnecessary.
///
/// # Example (immutable slice)
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, SplitByDiscriminant};
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B }
///
/// // compute discriminants on the fly
/// let data = [E::A(1), E::B, E::A(2)];
/// let a_disc = discriminant(&E::A(0));
///
/// let mut split: SplitByDiscriminant<_, &E> =
///     split_by_discriminant(&data[..], &[a_disc]);
/// assert_eq!(split.group(a_disc).unwrap().len(), 2);
///
/// // or store discriminants in constants for reuse
/// const A_DISC: std::mem::Discriminant<E> = discriminant(&E::A(0));
/// const B_DISC: std::mem::Discriminant<E> = discriminant(&E::B);
///
/// let mut split2: SplitByDiscriminant<_, &E> =
///     split_by_discriminant(&data[..], &[A_DISC, B_DISC]);
/// assert_eq!(split2.group(A_DISC).unwrap().len(), 2);
/// ```
/// Result of a discriminant split.
///
/// The struct is generic over *three* types to maximise flexibility:
///
/// * `T` – the enum (or other type) whose discriminant is used for
///   grouping.  Every discriminant key in `groups` has type
///   [`Discriminant<T>`].
/// * `G` – the type stored **inside the matching groups**.  This is usually
///   the iterator's item type, but with `map_by_discriminant` it can be a
///   transformed value (e.g. mapping `E` items to `String` summaries, or
///   storing `&mut i32` extracted from `&mut E`).
/// * `O` – the type stored in the “others” bucket.  It defaults to `G` so
///   the simple `split_by_discriminant` case is ergonomic, but you can make
///   it different when you want to treat unmatched items specially (for
///   example, mapping them to `()` or a count).
///
/// Having `G` and `O` distinct enables `map_by_discriminant` to return a
/// `SplitByDiscriminant<T, U, V>` where matched items become `U` and
/// unmatched ones become `V`.
///
/// # Examples
///
/// ```rust
/// # use split_by_discriminant::{split_by_discriminant, map_by_discriminant};
/// # use split_by_discriminant::SplitByDiscriminant;
/// # use std::mem::discriminant;
/// #[derive(Debug, PartialEq)] enum E { A(i32), B };
/// let a_disc = discriminant(&E::A(0));
///
/// // basic case where both types are the same
/// let data = [E::A(1), E::B];
/// let mut split: SplitByDiscriminant<_, &E> =
///     split_by_discriminant(&data[..], &[a_disc]);
/// assert_eq!(split.group(a_disc).unwrap()[0], &E::A(1));
///
/// // custom mapping: matched items → String, others → unit
/// let mut split2 = map_by_discriminant(&data[..], &[a_disc],
///     |e| format!("match:{:?}", e),
///     |_e| (),
/// );
/// assert_eq!(split2.group(a_disc).unwrap()[0], "match:A(1)");
/// ```

/// Actual struct definition (above examples reference it).
///
/// The struct is generic over *three* types to maximise flexibility:
///
/// * `T` – the enum (or other type) whose discriminant is used for
///   grouping.  Every discriminant key in `groups` has type
///   [`Discriminant<T>`].
/// * `G` – the type stored **inside the matching groups**.  This is usually
///   the iterator's item type, but with `map_by_discriminant` it can be a
///   transformed value (e.g. mapping `E` items to `String` summaries, or
///   storing `&mut i32` extracted from `&mut E`).
/// * `O` – the type stored in the “others” bucket.  It defaults to `G` so
///   the simple `split_by_discriminant` case is ergonomic, but you can make
///   it different when you want to treat unmatched items specially (for
///   example, mapping them to `()` or a count).
///
/// Having `G` and `O` distinct enables `map_by_discriminant` to return a
/// `SplitByDiscriminant<T, U, V>` where matched items become `U` and
/// unmatched ones become `V`.
///
pub struct SplitByDiscriminant<T, G, O = G> {
    groups: Map<Discriminant<T>, Vec<G>>,
    others: Vec<O>,
}

impl<T, G, O> SplitByDiscriminant<T, G, O> {
    /// Deconstruct into the owned collections.
    pub fn into_parts(self) -> (Map<Discriminant<T>, Vec<G>>, Vec<O>) {
        (self.groups, self.others)
    }

    /// Access the stored group for a discriminant.
    pub fn group(&mut self, id: Discriminant<T>) -> Option<&Vec<G>> {
        self.groups.get(&id)
    }
}

// implement extract only when we actually have mutable access to the inner T
// implement extract only when the group element type supports mutable
// borrowing of `T`.  the "others" type is irrelevant here.
impl<T, G, O> SplitByDiscriminant<T, G, O>
where
    G: BorrowMut<T>,
{
    /// Closure-based extraction that sidesteps the orphan rule.
    ///
    /// The caller supplies `f`, which maps `&mut T → Option<&mut U>`, so **no
    /// trait implementation is required**.  This is the recommended entry point
    /// when `T` or `U` come from external crates that you do not own.
    ///
    /// The closure must be valid for any lifetime `'a` (expressed as a
    /// higher-ranked trait bound), because Rust ties the output lifetime to
    /// the input borrow: `for<'a> FnMut(&'a mut T) -> Option<&'a mut U>`.
    /// In practice you never write this bound explicitly—closures that do
    /// straightforward pattern-matching satisfy it automatically.
    ///
    /// Returns `None` when `id` was not among the discriminants passed to
    /// [`split_by_discriminant`]; items for which `f` returns `None` are
    /// silently skipped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::split_by_discriminant;
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug)]
    /// enum E { A(i32), B }
    ///
    /// let mut data = [E::A(1), E::A(2), E::B];
    /// let a_disc = discriminant(&E::A(0));
    ///
    /// let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
    /// let ints: Vec<&mut i32> = split
    ///     .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    ///     .unwrap();
    /// assert_eq!(ints.len(), 2);
    /// ```
    ///
    /// # Crate-boundary pattern
    ///
    /// When `T` is foreign, define a plain helper function in an intermediary
    /// crate (`user_helper`) and pass it by name — no trait impl needed:
    ///
    /// ```rust
    /// use split_by_discriminant::split_by_discriminant;
    /// use std::mem::discriminant;
    ///
    /// // Imagine this lives in `user_helper`, depending on `external_enums`.
    /// // The function is local to *some* crate, so the orphan rule never fires.
    /// #[derive(Debug)] enum MyEnum { A(i32), B }
    /// fn extract_a(e: &mut MyEnum) -> Option<&mut i32> {
    ///     if let MyEnum::A(v) = e { Some(v) } else { None }
    /// }
    ///
    /// // user_downstream: pass the helper function directly — no closure syntax needed
    /// let mut data = [MyEnum::A(1), MyEnum::A(2), MyEnum::B];
    /// let a_disc = discriminant(&MyEnum::A(0));
    /// let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
    /// let ints: Vec<&mut i32> = split.extract_with(a_disc, extract_a).unwrap();
    /// assert_eq!(ints.len(), 2);
    /// ```
    ///
    /// For a higher-level API that binds the extractor up front and avoids
    /// repeating a closure on every call, see [`SplitWithExtractor`].
    pub fn extract_with<U, F>(&mut self, id: Discriminant<T>, mut f: F) -> Option<Vec<&mut U>>
    where
        F: for<'a> FnMut(&'a mut T) -> Option<&'a mut U>,
    {
        if let Some(vec) = self.groups.get_mut(&id) {
            let mut out = Vec::with_capacity(vec.len());
            for item in vec.iter_mut() {
                if let Some(u) = f(item.borrow_mut()) {
                    out.push(u);
                }
            }
            Some(out)
        } else {
            None
        }
    }
}

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
/// unwrap it with [`into_inner`] when you need consuming helpers.
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
/// A [`SplitByDiscriminant`] paired with a user‑supplied extractor value.
///
/// This wrapper carries exactly the same `T`, `G`, and `O` parameters as the
/// inner split (they describe the enum type and the element types for groups
/// and others).  The extra parameter `E` is the extractor type that implements
/// `ExtractFrom<T, U>` for one or more output types `U`; it allows the
/// ergonomic [`extract`] method to infer `U` without a closure at the call
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

/// Partition a sequence of elements into groups keyed by enum discriminant.
///
/// The `items` argument may be any iterator whose `Item` type is some value
/// `R` implementing `Borrow<T>` (typically `&T` or `&mut T`, but also `T`
/// itself).  Because of the blanket implementation of `Borrow<T>` for `T`,
/// owned iterators such as `Vec<T>::into_iter()` work seamlessly, allowing you
/// to take ownership of the elements.  Alternatively, pass `&[T]`, `&mut
/// Vec<T>`, or any other container that yields references, and the return type
/// will reflect the borrow kind.  When `R` is an immutable reference, helpers
/// like [`SplitByDiscriminant::extract_with`] are omitted via trait bounds.
///
/// Examples of accepted inputs:
///
/// * `&mut [T]`, `&mut Vec<T>` (yields `&mut T` items)
/// * `&[T]`, `&Vec<T>` (yields `&T` items)
/// * `Vec<T>::into_iter()` (yields `T` items)
///
/// A bare `[T]` cannot be passed by value because it is unsized; you always
/// borrow it.
///
/// The `kinds` argument is any iterable of values convertible to
/// `Discriminant<T>`; duplicates are discarded internally.
///
/// # Examples
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B(String), C }
///
/// // local extractor — no orphan rule issues
/// struct EExtract;
/// impl ExtractFrom<E, i32> for EExtract {
///     fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
///         if let E::A(v) = t { Some(v) } else { None }
///     }
/// }
///
/// let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
/// let a_disc = discriminant(&E::A(0));
/// let b_disc = discriminant(&E::B(String::new()));
///
/// // mutable slice — use SplitWithExtractor for ergonomic extraction
/// let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
/// let mut extractor = SplitWithExtractor::new(split, EExtract);
/// assert_eq!(extractor.group(a_disc).unwrap().len(), 2);
/// let ints: Vec<&mut i32> = extractor.extract(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
///
/// // or a mutable Vec
/// let mut vec = vec![E::A(3), E::C];
/// let mut split2 = split_by_discriminant(&mut vec, &[a_disc]);
/// assert_eq!(split2.group(a_disc).unwrap().len(), 1);
///
/// // owning iterator
/// let owned = vec![E::A(4), E::B(String::new())];
/// let mut split3 = split_by_discriminant(owned.into_iter(), &[a_disc]);
/// assert_eq!(split3.group(a_disc).unwrap().len(), 1);
/// ```
///
/// The return type reflects the borrow kind of the iterator.  Passing a
/// mutable container yields `SplitByDiscriminant<T, &mut T>`, an immutable
/// borrow gives `SplitByDiscriminant<T, &T>`, and an owning iterator results in
/// `SplitByDiscriminant<T, T>`.
/// 
/// # Example (custom mapping)
///
/// ```rust
/// use split_by_discriminant::map_by_discriminant;
/// use std::mem::discriminant;
///
/// #[derive(Debug)]
/// enum E { A(i32), B };
/// let a_disc = discriminant(&E::A(0));
/// let b_disc = discriminant(&E::B);
///
/// let data = [E::A(1), E::B];
/// let mut split = map_by_discriminant(&data[..], &[a_disc, b_disc],
///     |e| format!("match:{:?}", e),
///     |e| format!("other:{:?}", e),
/// );
/// assert_eq!(split.group(a_disc).unwrap()[0], "match:A(1)");
/// ```
///
/// The two closures control what happens to matched vs unmatched items
/// allowing the caller to transform values.
///
/// `map_by_discriminant` returns a `SplitByDiscriminant` where the
/// group and other element types may differ; this is what motivated the
/// additional type parameter on the struct itself.  The implementation is
/// identical to the old `split_internal` helper but inlined here so there is
/// only a single public function.
pub fn map_by_discriminant<T, I, K, R, U, V, M, N>(
    items: I,
    kinds: K,
    mut map_match: M,
    mut map_other: N,
) -> SplitByDiscriminant<T, U, V>
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
        .map(|k| k.borrow().clone())
        .collect();

    let mut groups: Map<Discriminant<T>, Vec<U>> = Map::new();
    let mut others: Vec<V> = Vec::new();

    for item in items.into_iter() {
        let d = std::mem::discriminant(item.borrow());
        if wanted.contains(&d) {
            groups.entry(d).or_default().push(map_match(item));
        } else {
            others.push(map_other(item));
        }
    }

    SplitByDiscriminant { groups, others }
}

/// Partition a sequence of elements into groups keyed by enum discriminant.
///
/// This is the simplest function in the library; it simply returns
/// `SplitByDiscriminant<T, R>` where the two element types coincide.
/// The bound `R: Borrow<T>` is required for the discriminant computation.
pub fn split_by_discriminant<T, I, K, R>(
    items: I,
    kinds: K,
) -> SplitByDiscriminant<T, R>
where
    I: IntoIterator<Item = R>,
    R: Borrow<T>,
    K: IntoIterator,
    K::Item: Borrow<Discriminant<T>>,
{
    map_by_discriminant(items, kinds, |r| r, |r| r)
}


impl<T, R> SplitByDiscriminant<T, R>
where
    R: Borrow<T>,
{
    /// Transform each group all at once, consuming `self`.
    ///
    /// The provided function is given ownership of the entire `Vec<R>` for
    /// each discriminant; it may convert them to some other representation.
    /// This is primarily a convenience for callers who want an immediate
    /// post‑processing step without manually iterating the map returned by
    /// [`SplitByDiscriminant::into_parts`].
    pub fn map_groups<U, F>(self, mut f: F) -> Map<Discriminant<T>, U>
    where
        F: FnMut(Vec<R>) -> U,
    {
        self.groups
            .into_iter()
            .map(|(k, v)| (k, f(v)))
            .collect()
    }

    /// Apply a transformation to the "others" vector.
    pub fn map_others<U, F>(self, f: F) -> U
    where
        F: FnOnce(Vec<R>) -> U,
    {
        f(self.others)
    }
}


#[cfg(test)]
mod tests;
