use crate::DiscriminantMap;

/// A simplified extraction trait for the common `&mut U` case.
///
/// Unlike [`ExtractFrom`], `Output` is an **associated type** rather than a type
/// parameter.  One extractor type can have at most one `SimpleExtractFrom<T>` impl
/// per `T`, which gives the compiler a unique `(E, T) → Output` mapping.  This
/// uniqueness allows [`SplitWithExtractor::extract`] to return a fully determined
/// type with no turbofish annotation required at the call site.
///
/// A blanket impl provides [`ExtractFrom<T, ()>`] automatically for every
/// `E: SimpleExtractFrom<T>`, so [`SplitWithExtractor::extract_gat::<()>`] and
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
/// let ints: Vec<&mut i32> = ex.extract_simple(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
/// ```
pub trait SimpleExtractFrom<T> {
    /// The owned content type.  The full reference `&'a mut Output` is always
    /// produced by [`extract_from`](SimpleExtractFrom::extract_from).  The
    /// `'static` bound means `Output` must not borrow from any shorter-lived
    /// region; for enum fields like `i32`, `String`, or `IpAddr` this is always
    /// satisfied.
    type Output: 'static;
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
/// let ints: Vec<&mut i32>    = ex.extract(a_disc).unwrap();
/// assert_eq!(ints.len(), 2);
/// drop(ints); // end the borrow before the next call
/// let strs: Vec<&mut String> = ex.extract(b_disc).unwrap();
/// assert_eq!(strs.len(), 1);
/// ```
///
/// **Blanket impl:** every `E: SimpleExtractFrom<T>` automatically implements
/// `VariantExtractFrom<T, <E as SimpleExtractFrom<T>>::Output>`, so if your extractor
/// covers only one variant via `SimpleExtractFrom`, you get [`SplitWithExtractor::extract`]
/// for free without any additional impl.
///
/// **Conflict note:** do **not** manually implement `VariantExtractFrom<T, U>` when `U` is
/// the same type as your `SimpleExtractFrom<T>::Output` — the blanket impl will fail to compile.
pub trait VariantExtractFrom<T, U> {
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

/// A [`DiscriminantMap`] with an extractor value bound up front.
///
/// This wrapper carries four generic parameters:
///
/// * `T` – the enum type whose discriminant keys the groups.
/// * `G` – element type stored in each group.
/// * `O` – element type for the others bucket (defaults to `G`).
/// * `E` – an extractor value implementing [`ExtractFrom<T, S>`] for one or
///   more selector types `S`.
///
/// Construct with [`SplitWithExtractor::new`] after calling
/// [`crate::split_by_discriminant`].
///
/// # Example
///
/// ```rust
/// use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
/// use std::mem::discriminant;
///
/// #[derive(Debug)] pub enum MyEnum { A(i32), B(String) }
///
/// struct MyExtractor;
/// pub struct SelectA;
/// pub struct SelectB;
///
/// impl ExtractFrom<MyEnum, SelectA> for MyExtractor {
///     type Output<'a> = &'a mut i32;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::A(v) = t { Some(v) } else { None }
///     }
/// }
/// impl ExtractFrom<MyEnum, SelectB> for MyExtractor {
///     type Output<'a> = &'a mut String;
///     fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<Self::Output<'a>> {
///         if let MyEnum::B(s) = t { Some(s) } else { None }
///     }
/// }
///
/// let mut data = vec![MyEnum::A(1), MyEnum::B("hi".into()), MyEnum::A(2)];
/// let a_disc = discriminant(&MyEnum::A(0));
/// let b_disc = discriminant(&MyEnum::B(String::new()));
///
/// let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
/// let mut extractor = SplitWithExtractor::new(split, MyExtractor);
///
/// { let v: Vec<&mut i32>    = extractor.extract_gat::<SelectA>(a_disc).unwrap(); assert_eq!(v.len(), 2); }
/// { let v: Vec<&mut String> = extractor.extract_gat::<SelectB>(b_disc).unwrap(); assert_eq!(v.len(), 1); }
/// ```
pub struct SplitWithExtractor<T, G, O, E> {
    pub(crate) inner: DiscriminantMap<T, G, O>,
    pub(crate) extractor: E,
}
