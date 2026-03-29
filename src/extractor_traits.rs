//! Extraction traits and their blanket implementations.

/// A simplified extraction trait for the common `&mut U` case.
///
/// Unlike [`ExtractFrom`], `Output` is an **associated type** rather than a type
/// parameter.  One extractor type can have at most one `SimpleExtractFrom<T>` impl
/// per `T`, which gives the compiler a unique `(E, T) → Output` mapping.  This
/// uniqueness allows [`SplitWithExtractor::as_mut_simple`] to return a fully determined
/// type with no turbofish annotation required at the call site.
///
/// A blanket impl provides [`ExtractFrom<T, ()>`] automatically for every
/// `E: SimpleExtractFrom<T>`, so [`SplitWithExtractor::as_mut_with::<()>`] and
/// [`SplitWithExtractor::take_extracted::<()>`] are also available for free.
///
/// Implement `SimpleExtractFrom<T>` for the common single-field `&mut U` case.
/// For multi-field outputs (tuples, named lifetime-carrying structs) or when you
/// need multiple extraction targets on the same `(E, T)` pair, implement
/// [`ExtractFrom<T, S>`] directly with a named selector `S`.
///
/// **Conflict note:** do **not** also implement `ExtractFrom<T, ()>` manually on a
/// type that already implements `SimpleExtractFrom<T>` — the blanket fills that slot
/// and a duplicate impl will fail to compile.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::{SimpleExtractFrom, split_by_discriminant, SplitWithExtractor};
/// use std::mem::discriminant;
///
/// #[derive(Debug, PartialEq)] enum E { A(i32), B }
/// struct EEx;
///
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
/// // No turbofish required:
/// let ints: Vec<&mut i32> = ex.as_mut_simple(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
/// ```
pub trait SimpleExtractFrom<T> {
    /// The owned content type.  The full reference `&'a mut Output` is always
    /// produced by [`extract_from`](SimpleExtractFrom::extract_from).  The
    /// `'static` bound means `Output` must not borrow from any shorter-lived
    /// region; for enum fields like `i32`, `String`, or `IpAddr` this is always
    /// satisfied.
    type Output: 'static;
    /// Extract a mutable reference to the inner field from `t`.
    ///
    /// # Invariant
    ///
    /// Implementations **must not change the discriminant** of `t`.  Writing
    /// `*t = DifferentVariant(...)` inside this method is valid Rust but
    /// leaves the discriminant map in an inconsistent state: the item remains
    /// in its original bucket despite now having a different variant.
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut Self::Output>;
}

/// The v0.4-style extraction trait for extracting a single field reference from a `&mut T`.
///
/// Unlike [`SimpleExtractFrom`] (which uses an *associated* `Output` type), `U` here is a
/// **type parameter**.  One extractor type can implement `VariantExtractFrom<T, _>` multiple
/// times for the same `T` — once per variant field type — making it possible to cover an
/// entire enum with a single extractor struct:
///
/// ```rust
/// use split_by_discriminant::{VariantExtractFrom, split_by_discriminant, SplitWithExtractor};
/// use std::mem::discriminant;
///
/// #[derive(Debug, PartialEq)] enum E { A(i32), B(String), C }
/// struct EEx;
///
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
/// // Both variants — U inferred from binding, no turbofish:
/// let ints: Vec<&mut i32>    = ex.as_mut(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
/// drop(ints); // end the borrow before the next call
/// let strs: Vec<&mut String> = ex.as_mut(b_disc).unwrap();
/// assert_eq!(strs.len(), 1);
/// ```
///
/// **Blanket impl:** every `E: SimpleExtractFrom<T>` automatically implements
/// `VariantExtractFrom<T, <E as SimpleExtractFrom<T>>::Output>`, so if your extractor
/// covers only one variant via `SimpleExtractFrom`, you get [`SplitWithExtractor::as_mut`]
/// for free without any additional impl.
///
/// **Conflict note:** do **not** manually implement `VariantExtractFrom<T, U>` when `U` is
/// the same type as your `SimpleExtractFrom<T>::Output` — the blanket impl will fail to compile.
pub trait VariantExtractFrom<T, U> {
    /// Extract a mutable reference to the inner field from `t`.
    ///
    /// # Invariant
    ///
    /// Implementations **must not change the discriminant** of `t`.  Writing
    /// `*t = DifferentVariant(...)` inside this method leaves the discriminant
    /// map in an inconsistent state.
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut U>;
}

/// Defines how to obtain a value from a `&mut T` using a selector type.
///
/// Unlike implementing a trait directly on `T`, the impl here lives on a
/// *local extractor type* that you define in your own crate.  This means the
/// orphan rule is never an issue, even when both `T` and the output come from
/// external crates.
///
/// The `Selector` type parameter disambiguates between multiple impls on the
/// same extractor for different output types (e.g., extracting `&mut i32` vs
/// `&mut String` from the same enum).  Use a ZST (e.g., `pub struct SelectA;`)
/// as the selector when you need to distinguish impls.  The default is `()`.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::ExtractFrom;
///
/// #[derive(Debug)] enum MyEnum { A(i32), B(String) }
///
/// pub struct MyExtractor;
/// pub struct SelectA;
/// pub struct SelectB;
///
/// impl ExtractFrom<MyEnum, SelectA> for MyExtractor {
///     type Output<'a> = &'a mut i32;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
///
/// impl ExtractFrom<MyEnum, SelectB> for MyExtractor {
///     type Output<'a> = &'a mut String;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::B(s) = t { Some(s) } else { None }
///     }
/// }
/// ```
pub trait ExtractFrom<T, Selector = ()> {
    type Output<'a>
    where
        T: 'a;
    /// Extract a value from `t` using this selector.
    ///
    /// # Invariant
    ///
    /// Implementations **must not change the discriminant** of `t`.  Writing
    /// `*t = DifferentVariant(...)` inside this method leaves the discriminant
    /// map in an inconsistent state.
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<Self::Output<'a>>;
}

/// Consuming counterpart of [`ExtractFrom`].
///
/// Where `ExtractFrom` reborrows via `&mut T` (shortening any inner lifetime),
/// `TakeFrom` receives `G` **by value** (moved), so any reference derived from
/// it carries the full original lifetime.
///
/// The `Selector` type parameter disambiguates impls in the same way as for
/// [`ExtractFrom`].
///
/// # `take_*` vs `as_mut_*`
///
/// Both families extract inner fields of a grouped enum, but they differ on
/// two axes: whether the group is removed, and how long the result lives.
///
/// | Family | Methods | Removes group? | Lifetime of result |
/// |--------|---------|----------------|--------------------|
/// | **reborrow** | [`SplitWithExtractor::as_mut_simple`], [`SplitWithExtractor::as_mut`], [`SplitWithExtractor::as_mut_with`] | No | tied to `&mut self` borrow |
/// | **move** | [`SplitWithExtractor::take_simple`], [`SplitWithExtractor::take_extracted`] | Yes | full original `'items` lifetime |
///
/// Use `take_*` when extracted references need to **outlive** the
/// [`SplitWithExtractor`] — for instance when returning them from a function
/// whose signature requires a lifetime established by the original slice borrow.
///
/// Use `as_mut_*` when you want to mutate or inspect a group in-place and
/// then leave it in the map for further operations (or put items back).
///
/// # Lifetime preservation
///
/// When `G = &'items mut T` (the typical result of calling
/// `split_by_discriminant(&mut data[..], ...)`) the `'items` lifetime is
/// already encoded in `Vec<G>`.  The `as_mut_*` methods re-borrow elements
/// from that `Vec`, introducing a new lifetime tied to the `&mut self` borrow
/// of the map — the result cannot outlive the map.
///
/// `TakeFrom::take_from(&self, g: G)` receives `g: &'items mut T` **by value**
/// (moved out of the `Vec`).  The reference is already borrowed for `'items`;
/// moving it rather than re-borrowing preserves that lifetime in the return
/// type.  The result can therefore be placed in a binding that outlives the
/// `SplitWithExtractor` itself.
///
/// See [`docs/lifetime-model.md`](../docs/lifetime-model.md) for annotated
/// examples of both patterns.
///
/// # Blanket implementation
///
/// A blanket impl is provided for every `E: ExtractFrom<T, S>`, covering the
/// `G = &'a mut T` case:
///
/// ```text
/// impl<'a, T, S, E: ExtractFrom<T, S>> TakeFrom<&'a mut T, S> for E
/// ```
///
/// This means you never need to implement `TakeFrom` manually if you have
/// already implemented [`ExtractFrom`]; the blanket impl makes your extractor
/// automatically compatible with [`SplitWithExtractor::take_extracted`].
///
/// # When to implement directly
///
/// Implement `TakeFrom<G, S>` directly only when `G` is **not** `&mut T` —
/// for example when `G` is an owned enum value from `map_by_discriminant`.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::{TakeFrom, split_by_discriminant, SplitWithExtractor};
/// use std::mem::discriminant;
///
/// #[derive(Debug)] enum E { A(i32), B }
/// struct OwnedEx;
/// impl TakeFrom<E> for OwnedEx {
///     type Output = i32;
///     fn take_from(&self, g: E) -> Option<i32> {
///         if let E::A(v) = g { Some(v) } else { None }
///     }
/// }
///
/// let data = vec![E::A(1), E::B, E::A(2)];
/// let a_disc = discriminant(&E::A(0));
/// let split = split_by_discriminant(data.into_iter(), &[a_disc]);
/// let mut ex = SplitWithExtractor::new(split, OwnedEx);
/// let values: Vec<i32> = ex.take_extracted::<()>(a_disc).unwrap();
/// assert_eq!(values, [1, 2]);
/// ```
pub trait TakeFrom<G, Selector = ()> {
    type Output;
    fn take_from(&self, g: G) -> Option<Self::Output>;
}

/// Immutable counterpart to [`SimpleExtractFrom`].
///
/// Like [`SimpleExtractFrom`], `Output` is an **associated type** giving the
/// compiler a unique `(E, T) → Output` mapping — at most one impl per
/// `(extractor, enum)` pair.  Unlike `SimpleExtractFrom`, the method takes
/// `&T` (not `&mut T`) and returns a shared reference `&Output`.
///
/// This enables read-only access to grouped items without needing a mutable
/// borrow.  A type may implement both `SimpleReadFrom<T>` and
/// `SimpleExtractFrom<T>` to expose both paths, or implement only
/// `SimpleReadFrom<T>` to make the mutable `as_mut_*` methods unavailable.
///
/// **No automatic blanket from `SimpleExtractFrom`:** because
/// `SimpleExtractFrom::extract_from` takes `&mut T`, there is no sound way to
/// derive `fn read_from(&T)` from it.  Types that already implement
/// `SimpleExtractFrom<T>` must implement `SimpleReadFrom<T>` separately — the
/// body is identical except the `mut` qualifiers are removed from the match arm.
///
/// **Free blankets provided:**  every `E: SimpleReadFrom<T>` automatically
/// receives:
/// - [`VariantReadFrom<T, Output>`] — enables [`SplitWithExtractor::as_ref`]
///   for the `Output` type without a turbofish.
/// - [`ReadFrom<T, ()>`] — enables [`SplitWithExtractor::as_ref_with::<()>`].
///
/// **Conflict note:** do **not** also implement `ReadFrom<T, ()>` or
/// `VariantReadFrom<T, Output>` manually on a type that already implements
/// `SimpleReadFrom<T>` — those slots are filled by the blankets.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::{SimpleReadFrom, split_by_discriminant, SplitWithExtractor};
/// use std::mem::discriminant;
///
/// #[derive(Debug)] enum E { A(i32), B }
/// struct EEx;
///
/// impl SimpleReadFrom<E> for EEx {
///     type Output = i32;
///     fn read_from<'a>(&self, t: &'a E) -> Option<&'a i32> {
///         if let E::A(v) = t { Some(v) } else { None }
///     }
/// }
///
/// let data = [E::A(1), E::A(2), E::B];
/// let a_disc = discriminant(&E::A(0));
/// let split = split_by_discriminant(&data[..], &[a_disc]);
/// let ex = SplitWithExtractor::new(split, EEx);
/// // No turbofish, no mutable borrow of ex:
/// let ints: Vec<&i32> = ex.as_ref_simple(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
/// ```
pub trait SimpleReadFrom<T> {
    /// The field type.  Must be `'static` so that `&'a Output` is valid for any
    /// caller-chosen lifetime `'a`.  For `i32`, `String`, `IpAddr` etc. this is
    /// always satisfied.  For lifetime-carrying outputs use [`ReadFrom<T, S>`]
    /// directly, where you control the GAT lifetime.
    type Output: 'static;
    fn read_from<'a>(&self, t: &'a T) -> Option<&'a Self::Output>;
}

/// Immutable counterpart to [`VariantExtractFrom`].
///
/// Like [`VariantExtractFrom`], `U` is a **type parameter** allowing multiple
/// impls on the same extractor for different field types.  Unlike
/// `VariantExtractFrom`, the method takes `&T` and returns `Option<&U>`.
///
/// **Blanket impl:** every `E: SimpleReadFrom<T>` automatically implements
/// `VariantReadFrom<T, <E as SimpleReadFrom<T>>::Output>`, so the primary output
/// type is covered without any additional impl.
///
/// **Conflict note:** do **not** manually implement `VariantReadFrom<T, U>` when
/// `U` is the same type as your `SimpleReadFrom<T>::Output`.
pub trait VariantReadFrom<T, U> {
    fn read_from<'a>(&self, t: &'a T) -> Option<&'a U>;
}

/// Immutable GAT counterpart to [`ExtractFrom`].
///
/// Like [`ExtractFrom`], the `Selector` type parameter disambiguates between
/// multiple impls on the same extractor.  The `Output<'a>` GAT carries the
/// borrow lifetime `'a` from `&'a T`, so multi-field tuple outputs and other
/// lifetime-carrying types are fully supported.
///
/// A blanket impl provides [`ReadFrom<T, ()>`] for every `E: SimpleReadFrom<T>`,
/// so the `()` selector slot is reserved — do not implement `ReadFrom<T, ()>`
/// manually alongside `SimpleReadFrom<T>`.
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::ReadFrom;
///
/// #[derive(Debug)] enum MyEnum { A(i32), B(String) }
///
/// pub struct MyExtractor;
/// pub struct SelectA;
///
/// impl ReadFrom<MyEnum, SelectA> for MyExtractor {
///     type Output<'a> = &'a i32 where MyEnum: 'a;
///     fn read_from<'a>(&self, t: &'a MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
/// ```
pub trait ReadFrom<T, Selector = ()> {
    type Output<'a>
    where
        T: 'a;
    fn read_from<'a>(&self, t: &'a T) -> Option<Self::Output<'a>>;
}

// ── Blanket implementations ───────────────────────────────────────────────────

/// Blanket: `SimpleExtractFrom<T>` → `ExtractFrom<T, ()>`
///
/// Coherent because `Output` is an associated type — every `(E, T)` pair maps to
/// exactly one `Output`, so at most one `ExtractFrom<T, ()>` is generated here.
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

/// Blanket: `SimpleExtractFrom<T>` → `VariantExtractFrom<T, Output>`
///
/// An extractor that implements `SimpleExtractFrom<T>` automatically gets the
/// `VariantExtractFrom<T, Output>` impl for free, so `SplitWithExtractor::as_mut()`
/// works on its associated variant without any additional impl.
/// Do not manually implement `VariantExtractFrom<T, U>` when `U == SimpleExtractFrom<T>::Output`.
impl<E, T> VariantExtractFrom<T, <E as SimpleExtractFrom<T>>::Output> for E
where
    E: SimpleExtractFrom<T>,
{
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut <E as SimpleExtractFrom<T>>::Output> {
        <E as SimpleExtractFrom<T>>::extract_from(self, t)
    }
}

/// Blanket: every `ExtractFrom<T, S>` is automatically a `TakeFrom<&'a mut T, S>`.
impl<'a, T, S, E> TakeFrom<&'a mut T, S> for E
where
    E: ExtractFrom<T, S>,
{
    type Output = E::Output<'a>;
    fn take_from(&self, g: &'a mut T) -> Option<Self::Output> {
        self.extract_from(g)
    }
}

/// Blanket: `SimpleReadFrom<T>` → `ReadFrom<T, ()>`
///
/// Coherent because `Output` is an associated type and types that implement
/// `SimpleReadFrom<T>` get exactly one `ReadFrom<T, ()>` impl.
/// Do not implement `ReadFrom<T, ()>` manually alongside `SimpleReadFrom<T>`.
impl<E, T> ReadFrom<T, ()> for E
where
    E: SimpleReadFrom<T>,
{
    type Output<'a>
        = &'a <E as SimpleReadFrom<T>>::Output
    where
        T: 'a;
    fn read_from<'a>(&self, t: &'a T) -> Option<Self::Output<'a>> {
        <E as SimpleReadFrom<T>>::read_from(self, t)
    }
}

/// Blanket: `SimpleReadFrom<T>` → `VariantReadFrom<T, Output>`
///
/// An extractor that implements `SimpleReadFrom<T>` automatically gets the
/// `VariantReadFrom<T, Output>` impl for free, so `SplitWithExtractor::as_ref()`
/// works on its associated variant without any additional impl.
/// Do not manually implement `VariantReadFrom<T, U>` when `U == SimpleReadFrom<T>::Output`.
impl<E, T> VariantReadFrom<T, <E as SimpleReadFrom<T>>::Output> for E
where
    E: SimpleReadFrom<T>,
{
    fn read_from<'a>(&self, t: &'a T) -> Option<&'a <E as SimpleReadFrom<T>>::Output> {
        <E as SimpleReadFrom<T>>::read_from(self, t)
    }
}

