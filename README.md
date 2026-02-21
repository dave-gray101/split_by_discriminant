# split_by_discriminant

`split_by_discriminant` is a lightweight Rust utility for partitioning a sequence of items by the discriminant of an `enum`. It is especially useful when you need to gather all values of a particular variant, operate on them, and then return them to the original collection.

## Primary API

### `split_by_discriminant`

Generic function that takes:

1. An iterable of items (`items`) whose element type `R` implements `Borrow<T>` (e.g. `&T`, `&mut T`, or `T`).
2. An iterable of discriminants (`kinds`) to match against; duplicates are ignored.

Returns a [`SplitByDiscriminant<T, R>`] containing:

- `groups`: a map from discriminant to a `Vec<R>` of matching items.
- `others`: a `Vec<R>` of items whose discriminant was not requested.

Type inference normally deduces the return type; you rarely need to annotate it explicitly.

### `SplitByDiscriminant` struct

Provides methods:

- `into_parts(self) -> (Map<Discriminant<T>, Vec<R>>, Vec<R>)` — consume the struct and obtain the groups and others.  
  The concrete map type is `HashMap` by default; enabling the `indexmap`
  feature switches to `IndexMap`/`IndexSet` instead.
- `group(&mut self, id: Discriminant<T>) -> Option<&Vec<R>>` — access a particular group by discriminant.

If `R` additionally implements `BorrowMut<T>`, you get a helper:

- `extract<U>(&mut self, id: Discriminant<T>) -> Option<Vec<&mut U>> where T: Extract<U>`

This method returns mutable references to inner values of the specified type for a given discriminant. The user is responsible for implementing the `Extract<U>` trait on their enum type to define how to borrow out the inner value of type `U`.

### `Extract` trait

```rust
pub trait Extract<U> {
    fn extract(&mut self) -> Option<&mut U>;
}
```

Implement this to enable `SplitByDiscriminant::extract` in mutable contexts. Typically you match against an enum variant and return a reference to its payload.

## Examples

```rust
use split_by_discriminant::{split_by_discriminant, SplitByDiscriminant, Extract};
use std::mem::discriminant;

#[derive(Debug)]
enum E { A(i32), B }

impl Extract<i32> for E {
    fn extract(&mut self) -> Option<&mut i32> {
        if let E::A(v) = self { Some(v) } else { None }
    }
}

let mut data = [E::A(1), E::B, E::A(2)];
let a_disc = discriminant(&E::A(0));

// borrow a mutable slice
let mut split: SplitByDiscriminant<_, &mut E> =
    split_by_discriminant(&mut data[..], &[a_disc]);

// access group and modify inner values
if let Some(group) = split.group(a_disc) {
    assert_eq!(group.len(), 2);
}

if let Some(mut vals) = split.extract::<i32>(a_disc) {
    vals[0] *= 10;
}
```

You can also pass an owned iterator:

```rust
let owned = vec![E::A(4), E::B];
let mut split3 = split_by_discriminant(owned.into_iter(), &[a_disc]);
```

Or use immutable references:

```rust
let data = [E::A(2), E::B];
let mut split2 = split_by_discriminant(&data, &[a_disc]);
assert_eq!(split2.group(a_disc).unwrap().len(), 1);
```

## Supported Inputs

- `&mut [T]` or `&mut Vec<T>` ⇒ `SplitByDiscriminant<T, &mut T>`
- `&[T]` or `&Vec<T>`         ⇒ `SplitByDiscriminant<T, &T>`
- Any owning iterator, e.g. `Vec<T>::into_iter()` ⇒ `R = T`.

## Notes

- You can precompute discriminants using `std::mem::discriminant` and store them in `const`s for reuse.
- Items not matching any requested discriminant are preserved in `others`, preserving original order.

## Testing

Unit and integration-style tests live in `src/tests.rs` demonstrating various use cases and ensuring API correctness.

---

This library emphasizes ergonomic partitioning based on enum discriminants with minimal trait bounds, making it compatible with a wide variety of container and ownership patterns.