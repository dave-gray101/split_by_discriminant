# split_by_discriminant

`split_by_discriminant` is a lightweight Rust utility for partitioning a sequence of items by the discriminant of an `enum`.

It provides two closely-related helpers:

* `split_by_discriminant` — the simple grouping operation.
* `map_by_discriminant` — a more flexible variant that applies separate
  mapping closures to matched and unmatched items, allowing you to change the
  output types on the fly.

Both are useful when you need to gather all values of a particular variant,
operate on them, and then return them to the original collection.

## Reborrow vs. move semantics

Two families of methods are provided for getting items out of a group:

| Family | Methods | Semantics |
|---|---|---|
| **Reborrow** | `extract`, `others` | The element stays in the map; a `&mut` reference into it is returned.  The returned lifetime is tied to the `&mut self` borrow, so it cannot outlive the map itself. |
| **Move (remove)** | `remove`, `remove_mapped`, `remove_with`, `take_simple`, `take_extracted`, `remove_others` | The group or others vector is removed from the map and returned by value.  When `G = &'items mut T` the returned `Vec` carries the full original `'items` lifetime, allowing it to outlive the map in which it was temporarily stored. |

Choose the **reborrow** family when you need multiple passes over the same group or when you will put the items back.  Choose the **move** family when you want to transfer ownership of the items to a longer-lived binding.

## Primary API

### `split_by_discriminant`

Generic function that takes:

1. An iterable of items (`items`) whose element type `R` implements `Borrow<T>` (e.g. `&T`, `&mut T`, or `T`).
2. An iterable of discriminants (`kinds`) to match against; duplicates are ignored.

Returns a `DiscriminantMap<T, R>` containing:

- `entries`: a map from discriminant to a `Vec<R>` of matching items.
- `others`: a `Vec<R>` of items whose discriminant was not requested.

Type inference normally deduces the return type; you rarely need to annotate it explicitly.

### `map_by_discriminant`

A more flexible variant of `split_by_discriminant` that accepts two mapping closures.
The first closure is applied to items whose discriminant is requested, and the second
handles all others.  This allows the types of grouped elements and the "others" bucket
to differ, and lets you perform on-the-fly transformations during partitioning.

### `DiscriminantMap<T, G, O>` struct

The result of a split operation.  Every parameter has a clear responsibility:

| Parameter | Role |
|-----------|------|
| `T` | The underlying enum (or any type with a `Discriminant`). Used to compute the map keys (`Discriminant<T>`) and for `Borrow<T>` bounds on input items. |
| `G` | Type stored inside each matching group. Defaults to the iterator's item type, but may be transformed by `map_by_discriminant` (e.g. `String`, `&mut i32`, etc.). |
| `O` | Type stored in the “others” bucket. Defaults to `G` to make the common case ergonomic, but you can choose a different type to handle unmatched items specially (e.g. map them to `()` or a count). |

The generic trio lets you express use cases where the group and
others types differ without resorting to `enum` or `Box<dyn>`.

Methods:

**Inspection**
- `others(&self)` — borrow the unmatched items as `&[O]`. Takes `&self`; safe to call without a mutable borrow.
- `get(&self, id)` — borrow a particular group by discriminant as `&[G]`.
- `get_mut(&mut self, id)` — mutably borrow a particular group as `&mut [G]`.

**Move (remove) — remove a group and take ownership of its elements**
- `remove(&mut self, id)` — remove and return the group as `Vec<G>`, preserving the full original lifetime when `G` is a reference.
- `remove_mapped<U>(&mut self, id, f: FnMut(G) -> U)` — remove a group and map every element through `f` by value; returns `Option<Vec<U>>`.
- `remove_with<U>(&mut self, id, f: FnMut(G) -> Option<U>)` — remove a group and filter-map every element through `f` by value; returns `Option<Vec<U>>`. Full lifetime preservation.
- `remove_others(&mut self)` — remove and return the others vector as `Vec<O>`. Unlike `into_parts`, `self` remains usable for further `remove*` calls afterward. A second call returns an empty `Vec`.

**Consuming**
- `into_parts(self)` — consume and return `(Map<Discriminant<T>, Vec<G>>, Vec<O>)`.
  The concrete map type is `HashMap` by default; enable the `indexmap` feature
  for `IndexMap`/`IndexSet` instead.
- `map_all(self, f)` — transform every group at once, consuming `self`.
- `map_others(self, f)` — transform the others vector, consuming `self`.

### Extraction Traits

Three traits handle different extraction scenarios:

- **`SimpleExtractFrom<T>`** — single-variant extractors with zero-annotation call site
- **`VariantExtractFrom<T, U>`** — multi-variant extractors with binding-inferred `U`
- **`ExtractFrom<T, Selector>`** — multi-field or complex outputs with explicit selector

**See [Four-Crate Pattern Guide](docs/four-crate-pattern-guide.md) for trait selection, implementation guidance, and decision trees.** The guide covers all traits, blanket impls, and patterns for factory-crate authors.


### `SplitWithExtractor<T, G, O, E>` struct

A thin wrapper around `DiscriminantMap` that pairs it with an extractor
value `E`.  The four type parameters serve these roles:

* `T` – the enum/`Discriminant` target, carried through from the inner split.
* `G` – group element type; forwarded from `DiscriminantMap`.
* `O` – others element type; also forwarded and defaults to `G` when the
  split is originally constructed.
* `E` – the extractor type that implements `ExtractFrom<T, S>` for one or more
  selector types `S`.  The extractor is usually a zero-sized local struct;
  its purpose is to give you a *constraint* that allows `extract::<S>` to
  disambiguate between multiple output types without a closure.  Because the
  impl lives on your local type, the orphan rule is satisfied even when `T`
  and the output are foreign.

With this design every parameter can vary independently and has a real use
case in the docs and tests.

Methods available directly on `SplitWithExtractor`:

**Inspection**
- `others` — forwarded from the inner split.
- `get` — forwarded from the inner split.
- `get_mut` — forwarded from the inner split.

**Move (remove) — remove a group and take ownership of its elements**
- `remove` — forwarded from the inner split; full lifetime preservation.
- `remove_mapped` — forwarded from the inner split.
- `remove_with` — forwarded from the inner split.
- `remove_others` — forwarded from the inner split.
- `take_simple(&mut self, id)` — consuming counterpart of `extract_simple`; requires `E: SimpleExtractFrom<T>`. No turbofish, no annotation — the return type is fully determined by `E` and `T`. Returned elements carry the full `'items` lifetime.
- `take_extracted<S>(&mut self, id)` — like `remove_with` but uses the bound extractor instead of a closure. Requires `E: TakeFrom<G, S>`, which is satisfied automatically for any `E: ExtractFrom<T, S>` when `G = &mut T`.

**Reborrow — borrow into a group without removing it**
- `extract<U>(&mut self, id)` — primary v0.4-style extraction; requires `E: VariantExtractFrom<T, U>`. `U` is inferred from the binding type on the receiving variable — no turbofish needed. Call once per variant in a separate scope so borrows do not overlap.
- `extract_simple(&mut self, id)` — fully annotation-free extraction; requires `E: SimpleExtractFrom<T>`. The return type is determined entirely by `E` and `T`, so not even a binding type annotation is needed.
- `extract_gat<S>(&mut self, id)` — extraction with an explicit selector; requires `E: ExtractFrom<T, S>`. Use this for multi-field outputs (tuples, named structs) or when `VariantExtractFrom` is not sufficient.

**Consuming**
- `into_inner(self) -> DiscriminantMap<T, G, O>` — unwrap to reach
  consuming methods (`into_parts`, `map_all`, `map_others`).

Construct with `SplitWithExtractor::new(split, extractor)`.

## Four-crate Pattern

The **factory crate pattern** solves the Rust orphan rule for extractors on foreign enums. A factory crate defines an extractor type and implements extraction traits; downstream callers then use it without needing to implement the traits themselves.

**See [Four-Crate Pattern Guide](docs/four-crate-pattern-guide.md) for detailed guidance, decision trees, and implementation examples.**

Quick example:

```rust
# use split_by_discriminant::split_by_discriminant;
# use std::mem::discriminant;
#[derive(Debug)]
enum E { A(i32), B }

let mut data = vec![E::A(1), E::B, E::A(2)];
let a_disc = discriminant(&E::A(0));

// move — returned refs carry full 'items lifetime
let ints: Vec<&mut i32> = {
    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    split.remove_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap()
};
assert_eq!(ints.len(), 2);
```

## Examples

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, VariantExtractFrom};
use std::mem::discriminant;

#[derive(Debug)]
enum E { A(i32), B(String), C }

struct EExtractor;

impl VariantExtractFrom<E, i32> for EExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}
impl VariantExtractFrom<E, String> for EExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut String> {
        if let E::B(s) = t { Some(s) } else { None }
    }
}

let mut data = vec![E::A(1), E::B("hello".into()), E::A(2), E::C];
let a_disc = discriminant(&E::A(0));
let b_disc = discriminant(&E::B(String::new()));

let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
let mut extractor = SplitWithExtractor::new(split, EExtractor);

// U inferred from binding — each call lives in its own scope so &mut borrows
// do not overlap.
{ let ints: Vec<&mut i32>    = extractor.extract(a_disc).unwrap(); assert_eq!(ints.len(), 2); }
{ let strs: Vec<&mut String> = extractor.extract(b_disc).unwrap(); assert_eq!(strs.len(), 1); }

// Consuming methods are reached via into_inner().
let (_, others) = extractor.into_inner().into_parts();
assert_eq!(others.len(), 1); // E::C
```

### Move-style extraction with full lifetime preservation

When you need the extracted references to outlive the `SplitWithExtractor`,
use `take_extracted`:

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom};
use std::mem::discriminant;

#[derive(Debug, PartialEq)]
enum E { A(i32), B }
struct EExtractor;
impl SimpleExtractFrom<E> for EExtractor {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

let mut data = [E::A(1), E::A(2), E::B];
let a_disc = discriminant(&E::A(0));

// ints outlives the SplitWithExtractor — full 'items lifetime preserved
let mut ints: Vec<&mut i32> = {
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);
    ex.take_extracted::<()>(a_disc).unwrap()
};

*ints[0] = 99;
drop(ints);
assert_eq!(data[0], E::A(99));
```

### `remove_mapped` — transform every element by value

```rust
use split_by_discriminant::split_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B }
let mut data = [E::A(1), E::A(2), E::B];
let a_disc = discriminant(&E::A(0));

let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
let labels: Vec<String> = split
    .remove_mapped(a_disc, |e| format!("{:?}", e))
    .unwrap();
assert_eq!(labels, ["A(1)", "A(2)"]);
```

### `remove_others` — retrieve unmatched items without consuming `self`

```rust
use split_by_discriminant::split_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B, C }
let mut data = [E::A(1), E::A(2), E::B, E::C];
let a_disc = discriminant(&E::A(0));

let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

// Remove the unmatched items — split remains usable.
let others: Vec<&mut E> = split.remove_others();
assert_eq!(others.len(), 2); // B and C

// Groups are still intact.
let group: Vec<&mut E> = split.remove(a_disc).unwrap();
assert_eq!(group.len(), 2); // A(1) and A(2)
```

### Other supported input types

You can also pass an owned iterator:

```rust
use split_by_discriminant::split_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B(String) }

let owned = vec![E::A(4), E::B(String::new())];
let a_disc = discriminant(&E::A(0));
let split = split_by_discriminant(owned.into_iter(), &[a_disc]);
let (groups, _) = split.into_parts();
assert_eq!(groups[&a_disc].len(), 1);
```

Or use immutable references (extraction not available on immutable refs):

```rust
use split_by_discriminant::{split_by_discriminant, DiscriminantMap};
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B(String) }

let data = [E::A(2), E::B(String::new())];
let a_disc = discriminant(&E::A(0));
let split: DiscriminantMap<_, &E> = split_by_discriminant(&data[..], &[a_disc]);
assert_eq!(split.get(a_disc).unwrap().len(), 1);
```

---

Use `map_by_discriminant` when you need to transform matched and unmatched
items during partitioning:

```rust
use split_by_discriminant::map_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)]
enum E { A(i32), B }

let data = [E::A(1), E::B];
let a_disc = discriminant(&E::A(0));
let b_disc = discriminant(&E::B);

let mut split = map_by_discriminant(&data[..], &[a_disc, b_disc],
    |e| format!("match:{:?}", e),
    |e| format!("other:{:?}", e),
);
assert_eq!(split.get(a_disc).unwrap(), &["match:A(1)".to_string()][..]);
```
## Proc Macros

`split_by_discriminant_macros` provides a derive macro that can implement the above logic in most simple cases.

### Quickstart: `#[derive(ExtractFrom)]`

The derive macro generates a zero-sized extractor type named `<EnumName>Extractor`. Use it with `SplitWithExtractor` to perform extraction without manually writing an extractor type.

```rust,ignore
use split_by_discriminant_macros::ExtractFrom;
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;

#[derive(Debug)]
enum E { A(i32), B }

#[derive(ExtractFrom)]
enum E { A(i32), B }

let mut data = vec![E::A(1), E::B];
let a_disc = discriminant(&E::A(0));

let split = split_by_discriminant(&mut data, &[a_disc]);
let mut extractor = SplitWithExtractor::new(split, EExtractor);
let ints: Vec<&mut i32> = extractor.extract(a_disc).unwrap();
```


### Customizing `#[derive(ExtractFrom)]` names

The derive macro supports a `#[extract_from(...)]` attribute to override the generated helper names.

#### Custom extractor name

By default `#[derive(ExtractFrom)]` generates a zero-sized extractor named `<EnumName>Extractor`.
Use:

```rust,ignore
use split_by_discriminant_macros::ExtractFrom;

#[derive(ExtractFrom)]
#[extract_from(extractor = "MyExtractor")]
enum E { A(i32) }
```

#### Custom selector name

When the derive must generate selector types (multi-field variants or duplicate field types), the default is `Select{Enum}{Variant}`.
You can override it on a per-variant basis or globally via a format string.

Per-variant override:

```rust,ignore
use split_by_discriminant_macros::ExtractFrom;

#[derive(ExtractFrom)]
enum E {
    #[extract_from(selector = "MySelector")]
    A(i32, String),
}
```

Global override (format string, supports `{}` or `{enum}`/`{variant}`):

```rust,ignore
use split_by_discriminant_macros::ExtractFrom;

#[derive(ExtractFrom)]
#[extract_from(selector = "Custom{enum}{variant}")]
enum E { A(i32, String) }
```

(The default format is `Select{}{}`, with the first `{}` substituted by the enum name and the second by the variant name.)



## Supported inputs

- `&mut [T]` or `&mut Vec<T>` → `DiscriminantMap<T, &mut T>`
- `&[T]` or `&Vec<T>` → `DiscriminantMap<T, &T>`
- Any owning iterator, e.g. `Vec<T>::into_iter()` → `R = T`

## Features

- **`indexmap`** — use `IndexMap`/`IndexSet` instead of `HashMap`/`HashSet`.
  Enables deterministic iteration order over groups.

## Documentation

- **[Four-Crate Pattern Guide](docs/four-crate-pattern-guide.md)** — Complete implementation guide for factory-crate authors. Covers all extraction traits, decision trees, blanket impls, and patterns.
- **[v0.4 to v0.5 Migration Guide](docs/v0.4-to-v0.5-guide.md)** — Upgrading from v0.4. Method renames and trait changes.

## Notes

- Discriminants can be precomputed with `std::mem::discriminant` and stored in `const`s for reuse.
- Items not matching any requested discriminant are preserved in `others` in original order.
- The `remove_*` methods work on any group element type, including owned values and immutable references.
- `remove_others` returns `Vec<O>` directly (not `Option`); a second call returns an empty `Vec`.
- Source code is human written and carefully reviewed - documentation and tests AI generated to keep them up to date.

## Testing

Integration tests and unit tests live in the `tests/` directory alongside `src/`