mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;

#[test]
fn split_with_extractor_and_extract() {
    // ComplexExtractor: SimpleExtractFrom<E> for A (annotation-free extract_simple)
    //                   VariantExtractFrom<E, String> for B (binding-inferred extract)
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    {
        let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
        let mut extractor = SplitWithExtractor::new(split, ComplexExtractor);

        // raw group access still available on SplitWithExtractor directly
        assert_eq!(extractor.get(a_disc).unwrap().len(), 2);

        // SimpleExtractFrom path — no annotation needed at all
        let mut ints = extractor.extract_simple(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] = 10;

        // VariantExtractFrom path — U inferred from binding, no turbofish
        let mut strings: Vec<&mut String> = extractor.extract(b_disc).unwrap();
        assert_eq!(strings.len(), 1);
        strings[0].push_str("!");
    }

    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::B("hi!".into()));
}

/// Two of three variants extracted in one pass using `ComplexExtractor`:
///
/// - `E::A(i32)`    via `extract()` — `U = i32` inferred from binding;
///   backed by `VariantExtractFrom<E, i32>` (provided for free by the
///   `SimpleExtractFrom<E>` blanket).
/// - `E::B(String)` via `extract()` — `U = String` inferred from binding;
///   backed by the explicit `VariantExtractFrom<E, String>` impl on
///   `ComplexExtractor`.
/// - `E::C` is not extracted at all; it lands in the others bucket untouched.
#[test]
fn extract_two_of_three_variants() {
    let mut data = [
        E::A(1), E::B("hello".into()), E::A(2),
        E::C,
        E::B("world".into()), E::A(3),
    ];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);

    // E::C was never requested — it lands in others untouched.
    assert_eq!(ex.others().len(), 1);

    // U = i32 inferred from binding; VariantExtractFrom<E, i32> via SimpleExtractFrom blanket
    {
        let ints: Vec<&mut i32> = ex.extract(a_disc).unwrap();
        assert_eq!(ints.len(), 3);
        for v in ints { *v *= 10; }
    }

    // U = String inferred from binding; explicit VariantExtractFrom<E, String> impl
    {
        let strings: Vec<&mut String> = ex.extract(b_disc).unwrap();
        assert_eq!(strings.len(), 2);
        for s in strings { s.push('!'); }
    }

    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::B("hello!".into()));
    assert_eq!(data[2], E::A(20));
    assert_eq!(data[3], E::C);
    assert_eq!(data[4], E::B("world!".into()));
    assert_eq!(data[5], E::A(30));
}

/// Both simple-field variants extracted from a `SplitWithExtractor` using
/// `extract_with` and inline closures.  No named extractor trait impls,
/// no selector types, no turbofish — identical ergonomics to v0.x.
///
/// The bound extractor (`SimpleExtractor`) is not involved in the extraction
/// at all; `extract_with` delegates directly to the inner `DiscriminantMap`.
#[test]
fn extract_with_closure_no_turbofish() {
    let mut data = [
        E::A(1), E::B("hello".into()), E::A(2),
        E::C,
        E::B("world".into()), E::A(3),
    ];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    // SimpleExtractor is required by the type but is not consulted by extract_with.
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    assert_eq!(ex.others().len(), 1);

    // U = i32 inferred from Vec<&mut i32> — no turbofish:
    {
        let ints: Vec<&mut i32> = ex
            .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
            .unwrap();
        assert_eq!(ints.len(), 3);
        for v in ints { *v *= 10; }
    }

    // U = String inferred independently — same method, different U:
    {
        let strings: Vec<&mut String> = ex
            .extract_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
            .unwrap();
        assert_eq!(strings.len(), 2);
        for s in strings { s.push('!'); }
    }

    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::B("hello!".into()));
    assert_eq!(data[2], E::A(20));
    assert_eq!(data[3], E::C);
    assert_eq!(data[4], E::B("world!".into()));
    assert_eq!(data[5], E::A(30));
}

/// `get_mut` on `SplitWithExtractor` returns a mutable slice of the stored group.
#[test]
fn get_mut_on_split_with_extractor() {
    let mut data = [E::A(10), E::A(20), E::B("x".into())];
    let a_disc = a_disc();

    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    {
        let group = ex.get_mut(a_disc).unwrap();
        assert_eq!(group.len(), 2);
        if let E::A(v) = group[0] { *v = 99; }
    }

    // absent discriminant returns None
    assert!(ex.get_mut(b_disc()).is_none());
    assert_eq!(data[0], E::A(99));
}

/// `extract_with` is also available directly on `DiscriminantMap`.
#[test]
fn extract_with_on_discriminant_map() {
    let mut data = [E::A(10), E::B("x".into()), E::A(20), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let mut map = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    let ints: Vec<&mut i32> = map
        .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    for v in ints { *v += 1; }

    let mut strs: Vec<&mut String> = map
        .extract_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strs.len(), 1);
    strs[0].push('!');

    assert_eq!(data[0], E::A(11));
    assert_eq!(data[1], E::B("x!".into()));
    assert_eq!(data[2], E::A(21));
}

