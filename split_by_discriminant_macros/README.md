# split_by_discriminant_macros

`split_by_discriminant_macros` is a companion crate to `split_by_discriminant` that provides procedural macros for deriving extractor types and generating helpers.

## Table of contents

- [Quickstart](#quickstart)
- [Macros](#macros)
  - [`#[derive(ExtractFrom)]`](#deriveextractfrom)
  - [`extract_from!`](#extract_from)
  - [`make_extractor!`](#make_extractor)
- [When to use what](#when-to-use-what)

## Quickstart

Add to `Cargo.toml`:

```toml
[dependencies]
split_by_discriminant = "^0.5"
split_by_discriminant_macros = "^0.5"
```

```rust
use split_by_discriminant::split_by_discriminant;
use split_by_discriminant_macros::ExtractFrom;
use std::mem::discriminant;

#[derive(Debug, PartialEq, ExtractFrom)]
enum E { A(i32), B(String), C }

fn main() {
    let mut values = vec![E::A(1), E::B("x".into()), E::C, E::A(2)];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(&mut values, &[a_disc]);
    let mut extractor = split_by_discriminant::SplitWithExtractor::new(split, EExtractor);

    // Mutable: as_mut_simple / as_mut require &mut
    let a_values: Vec<&mut i32> = extractor.as_mut(a_disc).unwrap();
    assert_eq!(a_values.len(), 2);
    drop(a_values);

    // Read-only: as_ref_simple / as_ref require only &self
    let a_refs: Vec<&i32> = extractor.as_ref(a_disc).unwrap();
    assert_eq!(a_refs.len(), 2);
}
```

## Macros

Use one of these macros depending on your workflow:

- `#[derive(ExtractFrom)]`: auto-generate extractor and selectors from an enum definition.
- `extract_from!`: function-like macro equivalent to derive, useful for macros or generated code.
- `make_extractor!`: creates a new utility function that calls split_by_discriminant::split_by_discriminant and split_by_discriminant::SplitWithExtractor with a specific extractor type

## When to use what

| Use case | Macro / method | Why |
|---|---|---|
| External enum with simple variants | `#[derive(ExtractFrom)]` | minimal handwritten code, best UX |
| Generated enum definitions | `extract_from!` | avoids derive macro restrictions |
| API boundary adapter | `make_extractor!` | creates helper function binding specific extractor types |



## `#[derive(ExtractFrom)]`

Produces an extractor type named `<EnumName>Extractor` by default.

### Options

- `#[extract_from(extractor = "MyExtractor")]` — use a custom extractor type name.
- `#[extract_from(selector = "MySelector")]` on a variant to customize selector type name.
- `#[extract_from(selector = "Custom{enum}{variant}")]` on the enum to customize selector naming across variants.
- `#[extract_from(skip_empty)]` on an empty enum to allow derive to compile without variants.

### Behavior

- For single-field variants with a unique field type across all variants, `VariantExtractFrom`
  and `VariantReadFrom` are implemented for that field type.
- For multi-field or repeated field-type variants, selector structs are generated (default
  style: `Select{Enum}{Variant}`) and both `ExtractFrom` and `ReadFrom` are implemented
  for those selectors.
- Read-only counterparts (`SimpleReadFrom`, `VariantReadFrom`, `ReadFrom`) are always
  generated alongside their mutable counterparts, enabling both `as_ref_*` and `as_mut_*`
  methods on the generated extractor.
- There is **no automatic blanket from `SimpleExtractFrom` → `SimpleReadFrom`**; the derive
  macro generates both explicitly.

## `extract_from!` macro

`extract_from!(...)` is a function-like macro with the same semantics and supported attributes as the derive macro, but is usable in contexts where derive is not desired. This is primarily intended for use with the soon-to-be-released `mudcrab` project.

Example:

```rust,no_run
use split_by_discriminant_macros::extract_from;

extract_from! {
    enum E { A(i32), B(String), C }
}

// Equivalent to #[derive(ExtractFrom)] with generated EExtractor and selectors.
```

## `make_extractor!` macro

`make_extractor!` generates a helper function and ties it to an extractor type.

Parameters:

- First argument: group name (required). Used in documentation, and for defaults below.
- `extractor = <ExtractorType>`: extractor type name (default: `<GroupName>Extractor`)
- `fn_name = <function_name>` / `function = <function_name>`: generated mutable helper function name (default: `make_<GroupName>_extractor`)
- `ref_fn_name = <function_name>` / `ref_function = <function_name>`: optional generated read-only companion function name (uses `Borrow<T>` instead of `BorrowMut<T>`)
- `trait_name = <TraitName>`: optional extra trait bound for the generated function signature

Example:

```rust,ignore
use split_by_discriminant_macros::make_extractor;

struct MyExtractor;
impl split_by_discriminant::VariantExtractFrom<E, i32> for MyExtractor {
    fn extract_from<'a>(&self, item: &'a mut E) -> Option<&'a mut i32> { /* ... */ }
}

make_extractor!(E, extractor = MyExtractor, fn_name = make_my_extractor, trait_name = MyTrait);
```

## Notes

- This crate is `proc-macro` only and does not expose runtime library API.

## Links

- Main crate: `split_by_discriminant`
- Guide: `docs/four-crate-pattern-guide.md`
