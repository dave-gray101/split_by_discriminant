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
        let mut ints = extractor.as_mut_simple(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] = 10;

        // VariantExtractFrom path — U inferred from binding, no turbofish
        let mut strings: Vec<&mut String> = extractor.as_mut(b_disc).unwrap();
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
        let ints: Vec<&mut i32> = ex.as_mut(a_disc).unwrap();
        assert_eq!(ints.len(), 3);
        for v in ints { *v *= 10; }
    }

    // U = String inferred from binding; explicit VariantExtractFrom<E, String> impl
    {
        let strings: Vec<&mut String> = ex.as_mut(b_disc).unwrap();
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
/// `map_as_mut` and inline closures.  No named extractor trait impls,
/// no selector types, no turbofish — identical ergonomics to v0.x.
///
/// The bound extractor (`SimpleExtractor`) is not involved in the extraction
/// at all; `map_as_mut` delegates directly to the inner `DiscriminantMap`.
#[test]
fn map_as_mut_closure_no_turbofish() {
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
            .map_as_mut(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
            .unwrap();
        assert_eq!(ints.len(), 3);
        for v in ints { *v *= 10; }
    }

    // U = String inferred independently — same method, different U:
    {
        let strings: Vec<&mut String> = ex
            .map_as_mut(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
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

/// `get_mut` on `SplitWithExtractor` returns a `GroupMut` for the stored group.
#[test]
fn get_mut_on_split_with_extractor() {
    let mut data = [E::A(10), E::A(20), E::B("x".into())];
    let a_disc = a_disc();

    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    {
        let mut group = ex.get_mut(a_disc).unwrap();
        assert_eq!(group.len(), 2);
        // Mutate the first element's field via iter_mut
        for item in group.iter_mut() {
            if let E::A(v) = item { *v = 99; break; }
        }
    }

    // absent discriminant returns None
    assert!(ex.get_mut(b_disc()).is_none());
    assert_eq!(data[0], E::A(99));
}

/// `for_each_group_mut` is delegated through `SplitWithExtractor` to the inner map.
#[test]
fn for_each_group_mut_on_split_with_extractor() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let mut a_len = 0usize;
    let mut b_seen = false;
    ex.for_each_group_mut(&[a_disc, b_disc], |disc, mut group| {
        if disc == a_disc {
            a_len = group.len();
            for item in group.iter_mut() {
                if let E::A(v) = item { *v *= 10; }
            }
        } else if disc == b_disc {
            b_seen = true;
        }
    });

    assert_eq!(a_len, 2);
    assert!(b_seen);
    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::A(20));
}

/// `map_as_mut` is also available directly on `DiscriminantMap`.
#[test]
fn map_as_mut_on_discriminant_map() {
    let mut data = [E::A(10), E::B("x".into()), E::A(20), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let mut map = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    let ints: Vec<&mut i32> = map
        .map_as_mut(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    for v in ints { *v += 1; }

    let mut strs: Vec<&mut String> = map
        .map_as_mut(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strs.len(), 1);
    strs[0].push('!');

    assert_eq!(data[0], E::A(11));
    assert_eq!(data[1], E::B("x!".into()));
    assert_eq!(data[2], E::A(21));
}
/// `others_mut` on `SplitWithExtractor` returns a mutable slice of the unmatched items.
#[test]
fn others_mut_on_split_with_extractor() {
    let mut data = [E::A(1), E::C, E::B("hi".into()), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    // C items are others; confirm mutable access
    assert_eq!(ex.others_mut().len(), 2);
    for item in ex.others_mut() {
        assert!(matches!(item, E::C));
    }
}

/// `extract_with` on `SplitWithExtractor` delegates to the inner `DiscriminantMap`.
#[test]
fn extract_with_on_split_with_extractor() {
    let mut data = [E::A(1), E::A(5), E::A(3)];
    let a_disc = a_disc();

    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Extract only A values >= 3 as owned integers
    let extracted = ex
        .extract_with(a_disc, |e| {
            if let E::A(v) = e { if *v >= 3 { Some(*v) } else { None } } else { None }
        })
        .unwrap();
    assert_eq!(extracted, vec![5, 3]);

    // Group is still present after extract_with
    assert_eq!(ex.get(a_disc).unwrap().len(), 3);

    // Absent discriminant returns None
    assert!(ex.extract_with(b_disc(), |_| Some(0u8)).is_none());
}
