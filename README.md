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
| **Reborrow** | `extract_with`, `extract`, `others` | The element stays in the split; a `&mut` reference into it is returned.  The returned lifetime is tied to the `&mut self` borrow, so it cannot outlive the split itself. |
| **Move (take)** | `take_group`, `take_group_mapped`, `take_group_with`, `take_extracted`, `take_others` | The group or others vector is removed from the split and returned by value.  When `G = &'items mut T` the returned `Vec` carries the full original `'items` lifetime, allowing it to outlive the split in which it was temporarily stored. |

Choose the **reborrow** family when you need multiple passes over the same group or when you will put the items back.  Choose the **move** family when you want to transfer ownership of the items to a longer-lived binding.

## Primary API

### `split_by_discriminant`

Generic function that takes:

1. An iterable of items (`items`) whose element type `R` implements `Borrow<T>` (e.g. `&T`, `&mut T`, or `T`).
2. An iterable of discriminants (`kinds`) to match against; duplicates are ignored.

Returns a `SplitByDiscriminant<T, R>` containing:

- `groups`: a map from discriminant to a `Vec<R>` of matching items.
- `others`: a `Vec<R>` of items whose discriminant was not requested.

Type inference normally deduces the return type; you rarely need to annotate it explicitly.

### `map_by_discriminant`

A more flexible variant of `split_by_discriminant` that accepts two mapping closures.
The first closure is applied to items whose discriminant is requested, and the second
handles all others.  This allows the types of grouped elements and the "others" bucket
to differ, and lets you perform on-the-fly transformations during partitioning.

### `SplitByDiscriminant<T, G, O>` struct

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
- `group(&mut self, id)` — borrow a particular group by discriminant.

**Move (take) — remove a group and take ownership of its elements**
- `take_group(&mut self, id)` — remove and return the group as `Vec<G>`, preserving the full original lifetime when `G` is a reference.
- `take_group_mapped<U>(&mut self, id, f: FnMut(G) -> U)` — remove a group and map every element through `f` by value; returns `Option<Vec<U>>`.
- `take_group_with<U>(&mut self, id, f: FnMut(G) -> Option<U>)` — remove a group and filter-map every element through `f` by value; returns `Option<Vec<U>>`. This is the consuming counterpart of `extract_with` with full lifetime preservation.
- `take_others(&mut self)` — remove and return the others vector as `Vec<O>`. Unlike `into_parts`, `self` remains usable for further `take_group*` calls afterward. A second call returns an empty `Vec` rather than an error.

**Reborrow — borrow into a group without removing it**
- `extract_with(&mut self, id, f)` — closure-based extraction; `f` maps `&mut T → Option<&mut U>`. Requires `G: BorrowMut<T>`. Returns `Option<Vec<&mut U>>` tied to the `&mut self` lifetime.

**Consuming**
- `into_parts(self)` — consume and return `(Map<Discriminant<T>, Vec<G>>, Vec<O>)`.
  The concrete map type is `HashMap` by default; enable the `indexmap` feature
  for `IndexMap`/`IndexSet` instead.
- `map_groups(self, f)` — transform every group at once, consuming `self`.
- `map_others(self, f)` — transform the others vector, consuming `self`.

### `ExtractFrom<T, U>` trait

```rust
pub trait ExtractFrom<T, U> {
    fn extract_from<'a>(&self, t: &'a mut T) -> Option<&'a mut U>;
}
```

Implement this on a **local extractor type** to describe how to borrow a `&mut U`
from a `&mut T`.  Because the impl is on *your* type (not on `T`), the orphan rule
is satisfied even when `T` and `U` both come from external crates.

### `TakeFrom<G, U>` trait

```rust
pub trait TakeFrom<G, U> {
    fn take_from(&self, g: G) -> Option<U>;
}
```

The consuming counterpart of `ExtractFrom`.  Where `ExtractFrom` reborrows via
`&mut T` (shortening any inner lifetime to the borrow), `TakeFrom` receives `G`
**by value** (moved), so any reference derived from it carries the full original
lifetime.

A **blanket implementation** is automatically provided for every `E: ExtractFrom<T, U>`,
covering the common `G = &mut T` case:

```text
impl<'a, T, U, E: ExtractFrom<T, U>> TakeFrom<&'a mut T, &'a mut U> for E { … }
```

This means you **never need to implement `TakeFrom` manually** if you have already
implemented `ExtractFrom`; the blanket impl makes your extractor automatically
compatible with `SplitWithExtractor::take_extracted`.

Implement `TakeFrom<G, U>` directly only when `G` is *not* `&mut T` — for example
when `G` is an owned value produced by `map_by_discriminant` and you want trait-based
extraction without a closure.

### `SplitWithExtractor<T, G, O, E>` struct

A thin wrapper around `SplitByDiscriminant` that pairs it with an extractor
value `E`.  The four type parameters serve these roles:

* `T` – the enum/`Discriminant` target, carried through from the inner split.
* `G` – group element type; forwarded from `SplitByDiscriminant`.
* `O` – others element type; also forwarded and defaults to `G` when the
  split is originally constructed.
* `E` – the extractor type that implements `ExtractFrom<T, U>` for one or more
  output types `U`.  The extractor is usually a zero-sized local struct;
  its purpose is to give you a *constraint* that allows `extract::<U>` to
  infer the right `U` without a closure.  Because the impl lives on your
  local type, the orphan rule is satisfied even when `T` and `U` are
  foreign.

With this design every parameter can vary independently and has a real use
case in the docs and tests.

Methods available directly on `SplitWithExtractor`:

**Inspection**
- `others` — forwarded from the inner split.
- `group` — forwarded from the inner split.

**Move (take) — remove a group and take ownership of its elements**
- `take_group` — forwarded from the inner split; full lifetime preservation.
- `take_group_mapped` — forwarded from the inner split.
- `take_group_with` — forwarded from the inner split.
- `take_others` — forwarded from the inner split.
- `take_extracted<U>(&mut self, id)` — like `take_group_with` but uses the bound extractor instead of a closure. Requires `E: TakeFrom<G, U>`, which is satisfied automatically for any `E: ExtractFrom<T, U>` when `G = &mut T`.

**Reborrow — borrow into a group without removing it**
- `extract_with` — forwarded from the inner split.
- `extract<U>(&mut self, id)` — ergonomic extraction via the bound extractor; requires `E: ExtractFrom<T, U>`.

**Consuming**
- `into_inner(self) -> SplitByDiscriminant<T, G, O>` — unwrap to reach
  consuming methods (`into_parts`, `map_groups`, `map_others`).

Construct with `SplitWithExtractor::new(split, extractor)`.

## Four-crate pattern (foreign enums)

The orphan rule prevents implementing a trait from crate A on a type from crate B
inside a third crate C.  `SplitWithExtractor` + `ExtractFrom` sidestep this.  The
following doctest demonstrates the same idea using a standard‑library enum as the
"foreign" type so you can see that anything – even `std` types – works.

```rust
// the "foreign" enum comes from `std` rather than a local module
use std::net::{IpAddr, Ipv4Addr};

// pretend `user_helper` is another crate that provides an extractor type
mod user_helper {
    use split_by_discriminant::ExtractFrom;
    use std::net::{IpAddr, Ipv4Addr};

    pub struct IpExtractor;
    impl ExtractFrom<IpAddr, Ipv4Addr> for IpExtractor {
        fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv4Addr> {
            if let IpAddr::V4(v4) = t { Some(v4) } else { None }
        }
    }
}

// --- user_downstream ------------------------------------------------------
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;
use user_helper::IpExtractor;

let mut data = vec![
    IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
    IpAddr::V6("::1".parse().unwrap()),
    IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)),
];
let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

// reborrow style (returned refs tied to &mut extractor)
let split = split_by_discriminant(&mut data, &[v4_disc]);
let mut extractor = SplitWithExtractor::new(split, IpExtractor);
let v4s: Vec<&mut Ipv4Addr> = extractor.extract(v4_disc).unwrap();
assert_eq!(v4s.len(), 2);

// move style (returned refs carry full 'items lifetime)
let split = split_by_discriminant(&mut data, &[v4_disc]);
let v4s: Vec<&mut Ipv4Addr> = {
    let mut extractor = SplitWithExtractor::new(split, IpExtractor);
    extractor.take_extracted(v4_disc).unwrap()
};
assert_eq!(v4s.len(), 2);
```
The `TakeFrom` blanket impl ships with this crate, so `take_extracted` works
automatically for every `ExtractFrom` impl without any extra code on your part.

For a one-off extraction without setting up an extractor type, pass a closure
directly to `extract_with` (reborrow) or `take_group_with` (move).

```rust
# use split_by_discriminant::split_by_discriminant;
# use std::mem::discriminant;
#[derive(Debug)]
enum E { A(i32), B }

let mut data = vec![E::A(1), E::B, E::A(2)];
let a_disc = discriminant(&E::A(0));

// reborrow — returned refs tied to &mut split's lifetime
let mut split = split_by_discriminant(&mut data, &[a_disc]);
let ints: Vec<&mut i32> = split
    .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
    .unwrap();
assert_eq!(ints.len(), 2);

// move — returned refs carry full 'items lifetime
let ints: Vec<&mut i32> = {
    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    split.take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap()
};
assert_eq!(ints.len(), 2);
```

## Examples

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
use std::mem::discriminant;

#[derive(Debug)]
enum E { A(i32), B(String), C }

struct EExtractor;
impl ExtractFrom<E, i32> for EExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

let mut data = vec![E::A(1), E::B("hello".into()), E::A(2), E::C];
let a_disc = discriminant(&E::A(0));
let b_disc = discriminant(&E::B(String::new()));

let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
let mut extractor = SplitWithExtractor::new(split, EExtractor);

// Reborrow extraction — each call lives in its own scope so &mut borrows
// do not overlap.
{
    let ints: Vec<&mut i32> = extractor.extract(a_disc).unwrap();
    assert_eq!(ints.len(), 2);
}

// Consuming methods are reached via into_inner().
let (groups, others) = extractor.into_inner().into_parts();
assert_eq!(others.len(), 1); // E::C
```

### Move-style extraction with full lifetime preservation

When you need the extracted references to outlive the `SplitWithExtractor`,
use `take_extracted` (or `take_group_with` for one-off closures):

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
use std::mem::discriminant;

#[derive(Debug, PartialEq)]
enum E { A(i32), B }
struct EExtractor;
impl ExtractFrom<E, i32> for EExtractor {
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
    ex.take_extracted(a_disc).unwrap()
};

*ints[0] = 99;
drop(ints);
assert_eq!(data[0], E::A(99));
```

### `take_group_mapped` — transform every element by value

```rust
use split_by_discriminant::split_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B }
let mut data = [E::A(1), E::A(2), E::B];
let a_disc = discriminant(&E::A(0));

let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
let labels: Vec<String> = split
    .take_group_mapped(a_disc, |e| format!("{:?}", e))
    .unwrap();
assert_eq!(labels, ["A(1)", "A(2)"]);
```

### `take_others` — retrieve unmatched items without consuming `self`

```rust
use split_by_discriminant::split_by_discriminant;
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B, C }
let mut data = [E::A(1), E::A(2), E::B, E::C];
let a_disc = discriminant(&E::A(0));

let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

// Take the unmatched items — split remains usable.
let others: Vec<&mut E> = split.take_others();
assert_eq!(others.len(), 2); // B and C

// Groups are still intact.
let group: Vec<&mut E> = split.take_group(a_disc).unwrap();
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
use split_by_discriminant::{split_by_discriminant, SplitByDiscriminant};
use std::mem::discriminant;

#[derive(Debug)] enum E { A(i32), B(String) }

let data = [E::A(2), E::B(String::new())];
let a_disc = discriminant(&E::A(0));
let mut split: SplitByDiscriminant<_, &E> = split_by_discriminant(&data[..], &[a_disc]);
assert_eq!(split.group(a_disc).unwrap().len(), 1);
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
assert_eq!(split.group(a_disc).unwrap(), &vec!["match:A(1)".to_string()]);
```

## Supported inputs

- `&mut [T]` or `&mut Vec<T>` → `SplitByDiscriminant<T, &mut T>`
- `&[T]` or `&Vec<T>` → `SplitByDiscriminant<T, &T>`
- Any owning iterator, e.g. `Vec<T>::into_iter()` → `R = T`

## Features

- **`indexmap`** — use `IndexMap`/`IndexSet` instead of `HashMap`/`HashSet`.
  Enables deterministic iteration order over groups.

## Notes

- Discriminants can be precomputed with `std::mem::discriminant` and stored in `const`s for reuse.
- Items not matching any requested discriminant are preserved in `others` in original order.
- `extract_with` and `SplitWithExtractor::extract` are only available when the group element
  type implements `BorrowMut<T>` (i.e. `&mut T` or `T` itself).
- The `take_*` methods do not require `BorrowMut<T>` — they work on any `G`, including owned
  values and immutable references.
- `take_others` returns `Vec<O>` directly (not `Option`); a second call returns an empty `Vec`.
- `take_extracted` requires `E: TakeFrom<G, U>`.  This is satisfied automatically when
  `G = &mut T` and `E: ExtractFrom<T, U>` by the blanket impl shipped with this crate.

## Testing

Integration tests and unit tests live in the `tests/` directory alongside `src/`:

- `tests/basic.rs` — core `SplitByDiscriminant` and `extract_with` behaviour.
- `tests/extractor.rs` — `SplitWithExtractor::extract`.
- `tests/foreign_workflow.rs` — end-to-end four-crate pattern using `std::net::IpAddr` as a real foreign enum.
- `tests/take_group.rs` — all `take_group*` and `take_extracted` methods, including lifetime-preservation proofs.