use std::mem::Discriminant;
use std::borrow::{Borrow, BorrowMut};

// bring the crate-local map/set aliases into scope
use crate::{Map, Set};

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

    /// Remove and return the owned group for `id`, if present.
    ///
    /// Unlike [`group`](SplitByDiscriminant::group), which reborrows through
    /// `&mut self` and therefore shortens any inner lifetime, this method
    /// **moves** the `Vec<G>` out of the map and returns it with its original
    /// lifetime intact.  This is the key difference when `G = &'items mut T`:
    /// the caller receives `Vec<&'items mut T>` instead of
    /// `Vec<&'borrow mut T>`, making it possible to store the result in a
    /// struct or return it from a function that expects the full `'items`
    /// lifetime.
    ///
    /// Calling `take_group` twice for the same discriminant returns `None` on
    /// the second call because the group has already been removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use split_by_discriminant::split_by_discriminant;
    /// use std::mem::discriminant;
    ///
    /// #[derive(Debug)] enum E { A(i32), B }
    /// let mut data = [E::A(1), E::A(2), E::B];
    /// let a_disc = discriminant(&E::A(0));
    ///
    /// let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
    /// let group: Vec<&mut E> = split.take_group(a_disc).unwrap();
    /// assert_eq!(group.len(), 2);
    /// // second call returns None — group was consumed
    /// assert!(split.take_group(a_disc).is_none());
    /// ```
    pub fn take_group(&mut self, id: Discriminant<T>) -> Option<Vec<G>> {
        self.groups.remove(&id)
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
