# split_by_discriminant

`split_by_discriminant` is a lightweight Rust utility for partitioning a sequence of items by the discriminant of an `enum`.

It provides two closely-related helpers:

* `split_by_discriminant` — the simple grouping operation.
* `map_by_discriminant` — a more flexible variant that applies separate
  mapping closures to matched and unmatched items, allowing you to change the
  output types on the fly.

Both are useful when you need to gather all values of a particular variant,
operate on them, and then return them to the original collection.

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

The result of a split.  `G` is the group element type and `O` is the "others"
element type (defaults to `G` for `split_by_discriminant`).

Methods:

- `into_parts(self)` — consume and return `(Map<Discriminant<T>, Vec<G>>, Vec<O>)`.
  The concrete map type is `HashMap` by default; enable the `indexmap` feature
  for `IndexMap`/`IndexSet` instead.
- `group(&mut self, id)` — borrow a particular group by discriminant.
- `extract_with(&mut self, id, f)` — closure-based extraction of inner values;
  `f` maps `&mut T → Option<&mut U>`. Requires `G: BorrowMut<T>`.
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

### `BoundSplit<T, G, O, E>` struct

Wraps a `SplitByDiscriminant` with a bound extractor `E`.  Provides an
ergonomic `extract(disc)` that requires no closure at the call site —
`U` is resolved via `E: ExtractFrom<T, U>`.

Methods available directly on `BoundSplit`:

- `group` — forwarded from the inner split.
- `extract_with` — forwarded from the inner split.
- `extract<U>(&mut self, id)` — ergonomic extraction via the bound extractor.
- `into_inner(self) -> SplitByDiscriminant<T, G, O>` — unwrap to reach
  consuming methods (`into_parts`, `map_groups`, `map_others`).

Construct with `BoundSplit::new(split, extractor)`.

## Four-crate pattern (foreign enums)

The orphan rule prevents implementing a trait from crate A on a type from crate B
inside a third crate C.  `BoundSplit` + `ExtractFrom` sidestep this completely:

| Crate | Role |
|---|---|
| `external_enums` | Defines `MyEnum`. Cannot be changed. |
| `split_by_discriminant` | This crate. |
| `user_helper` | Defines a **local** `MyEnumExtractor` and implements `ExtractFrom<MyEnum, _>` on it. Written once, reused everywhere. |
| `user_downstream` | Calls `BoundSplit::extract` — no trait impl needed. |

```rust
// user_helper
use split_by_discriminant::ExtractFrom;
use external_enums::MyEnum;   // foreign — cannot be changed

pub struct MyEnumExtractor;   // LOCAL type — orphan rule satisfied

impl ExtractFrom<MyEnum, i32> for MyEnumExtractor {
    fn extract_from<'a>(&self, t: &'a mut MyEnum) -> Option<&'a mut i32> {
        if let MyEnum::A(v) = t { Some(v) } else { None }
    }
}

// user_downstream
use split_by_discriminant::{split_by_discriminant, BoundSplit};
use user_helper::MyEnumExtractor;

let split = split_by_discriminant(&mut data, &[a_disc]);
let mut bound = BoundSplit::new(split, MyEnumExtractor);
let ints: Vec<&mut i32> = bound.extract(a_disc).unwrap();
```

For a one-off extraction without setting up an extractor type, pass a closure
directly to `extract_with`:

```rust
let ints: Vec<&mut i32> = split
    .extract_with(a_disc, |e| if let MyEnum::A(v) = e { Some(v) } else { None })
    .unwrap();
```

## Examples

```rust
use split_by_discriminant::{split_by_discriminant, BoundSplit, ExtractFrom};
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
let mut bound = BoundSplit::new(split, EExtractor);

// Ergonomic extraction — no closure at the call site.
// Each call lives in its own scope so &mut borrows do not overlap.
{
    let ints: Vec<&mut i32> = bound.extract(a_disc).unwrap();
    assert_eq!(ints.len(), 2);
}

// Consuming methods are reached via into_inner().
let (groups, others) = bound.into_inner().into_parts();
assert_eq!(others.len(), 1); // E::C
```

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
- `extract_with` and `BoundSplit::extract` are only available when the group element
  type implements `BorrowMut<T>` (i.e. `&mut T` or `T` itself).

## Testing

Unit and integration-style tests live in `src/tests.rs`, including a
`foreign_enum_workflow` module that demonstrates the four-crate pattern using
`std::net::IpAddr` as a real foreign enum.