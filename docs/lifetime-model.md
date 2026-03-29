# Lifetime Model

This document explains how lifetimes flow through `split_by_discriminant` and
why two extraction families — `as_mut_*` (reborrow) and `take_*` (move) — exist.

## The two extraction patterns

### 1. Reborrow (`as_mut_*`, `map_as_mut*`, `map_as_ref*`)

These methods hold the group inside the map and lend you references into it.
The returned `Vec<&mut U>` is tied to the `&mut self` borrow of
`SplitWithExtractor`, which means the references cannot escape the scope in
which you hold that mutable borrow.

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom};
use std::mem::discriminant;

#[derive(Debug, PartialEq)] enum E { A(i32), B }
struct EEx;
impl SimpleExtractFrom<E> for EEx {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

let mut data = [E::A(1), E::A(2), E::B];
let a_disc = discriminant(&E::A(0));

let split = split_by_discriminant(&mut data[..], &[a_disc]);
let mut ex = SplitWithExtractor::new(split, EEx);

// This block is necessary: ints borrows *ex mutably,
// so ex cannot be used again until ints is dropped.
{
    let ints: Vec<&mut i32> = ex.as_mut_simple(a_disc).unwrap();
    *ints[0] = 10; // mutate in-place — item stays in the map
}

// ex is still alive and can be used further:
let remaining: Vec<&mut i32> = ex.as_mut_simple(a_disc).unwrap();
assert_eq!(*remaining[0], 10);
```

The lifetime annotation would be written as:

```text
fn as_mut_simple<'s>(&'s mut self, ...) -> Option<Vec<&'s mut Output>>
```

`'s` starts when `as_mut_simple` is called and ends when the `Vec` is dropped.
It cannot extend beyond the `SplitWithExtractor`'s scope.

### 2. Move / preserve (`take_*`)

These methods remove the group from the map entirely, moving each `G` element
out of its `Vec`.  When `G = &'items mut T` (the typical result of
`split_by_discriminant(&mut data[..], ...)`), the references already carry the
full `'items` lifetime that was established when `data` was borrowed.  Moving
them by value — rather than re-borrowing through the map — preserves that
lifetime in the return type.

```rust
use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom};
use std::mem::discriminant;

#[derive(Debug, PartialEq)] enum E { A(i32), B }
struct EEx;
impl SimpleExtractFrom<E> for EEx {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

let mut data = [E::A(1), E::A(2), E::B];
let a_disc = discriminant(&E::A(0));

// ints is declared outside the SplitWithExtractor scope:
let ints: Vec<&mut i32> = {
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EEx);
    ex.take_simple(a_disc).unwrap()
    // ex is dropped here — but ints survives because it holds
    // references with the outer 'items lifetime, not 'ex.
};

// ints lives here, after ex is gone:
*ints[0] = 99;
drop(ints); // release the &mut data borrow

assert_eq!(data[0], E::A(99));
```

The lifetime annotation of `TakeFrom::take_from`:

```text
fn take_from(&self, g: G) -> Option<Self::Output>
//                   ^
//           g: &'items mut T — moved in, not re-borrowed
```

Because `g` is consumed by value, `'items` flows into `Output` rather than
being replaced by a shorter lifetime.

## Why the split matters

The reborrow pattern is necessary for iterative or multi-pass algorithms where
you want to mutate values and then put them back (or run more operations on the
same group).  The map continues to own the items.

The move pattern is necessary when the processing code and the data live in
different scopes — for example when a helper function extracts references and
returns them to a longer-lived caller, or when extracted references must be
stored in a struct.

## Choosing between the two patterns

| You want to … | Use |
|---|---|
| Mutate items in-place, keep them in the map | `as_mut_simple`, `as_mut<U>`, `as_mut_with<S>`, `map_as_mut` |
| Read items without any mutation | `map_as_ref`, `map_as_ref_multiple` |
| Extract ownership and keep using the map for other groups | `take_simple` / `take_extracted<S>` |
| Extract ownership and discard the map | `remove` / `remove_with` / `into_parts` |
| Return extracted refs from a function with a longer lifetime | `take_simple` / `take_extracted<S>` |

## The `extract_multiple_with` bridge

`extract_multiple_with(ids, f)` borrows mutably (like `as_mut*`) but the closure
returns an **owned** `U` (like `take*`).  It bridges the two patterns: groups
stay in the map, but extracted values are owned.  Use it when you derive an
intermediate owned value from each item without needing to carry the original
reference.
