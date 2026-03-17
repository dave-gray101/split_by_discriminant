// Tests for take_simple, take_extracted, and the TakeFrom trait abstraction.
// These tests verify consuming extraction that removes items from the split
// and returns references/values with the full lifetime.

mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, SimpleExtractFrom, ExtractFrom, TakeFrom};
use std::mem::discriminant;

// ── take_simple tests ─────────────────────────────────────────────────────────

/// A simple extractor for testing take_simple.
/// The blanket impl for TakeFrom<&'a mut T, ()> is automatically provided
/// by the impl of E: ExtractFrom<T, ()> (which SimpleExtractFrom provides).
struct TakeSimpleExtractor;

impl SimpleExtractFrom<E> for TakeSimpleExtractor {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

#[test]
fn take_simple_consuming_extractor() {
    // Test that take_simple consumes and returns owned values
    let a_disc = discriminant(&E::A(0));
    let mut data = [E::A(10), E::A(20), E::A(30), E::B("hi".into())];

    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, TakeSimpleExtractor);

    // take_simple should return references that can outlive the extractor
    let mut ints: Vec<&mut i32> = {
        ex.take_simple(a_disc).expect("should extract i32 values")
    };

    assert_eq!(ints.len(), 3);
    assert_eq!(*ints[0], 10);
    assert_eq!(*ints[1], 20);
    assert_eq!(*ints[2], 30);

    // Modify the values
    for v in &mut ints {
        **v *= 10;
    }

    assert_eq!(data[0], E::A(100));
    assert_eq!(data[1], E::A(200));
    assert_eq!(data[2], E::A(300));
}

#[test]
fn take_simple_returns_none_for_absent_discriminant() {
    // take_simple should return None for discriminants not in the split
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    let mut data = [E::A(1), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, TakeSimpleExtractor);

    // c_disc was not included in the split
    let result = ex.take_simple(c_disc);
    assert!(result.is_none());
}

#[test]
fn take_simple_removes_group_after_extraction() {
    // Calling take_simple should remove the group, preventing re-extraction
    let a_disc = discriminant(&E::A(0));
    let mut data = [E::A(1), E::A(2)];

    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, TakeSimpleExtractor);

    // First take succeeds
    let _first = ex.take_simple(a_disc).expect("first take should work");

    // Second take on same discriminant should return None (group already removed)
    let second = ex.take_simple(a_disc);
    assert!(second.is_none());
}

// ── take_extracted tests ──────────────────────────────────────────────────────

/// `take_extracted` uses the bound `ExtractFrom` impl (via the `TakeFrom`
/// blanket) and removes the group.
#[test]
fn take_extracted_basic() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);

    let ints: Vec<&mut i32> = ex.take_extracted::<()>(a_disc).unwrap();
    assert_eq!(ints.len(), 2);

    // group consumed — second call returns None
    assert!(ex.take_extracted::<()>(a_disc).is_none());

    // other group still accessible
    let strings: Vec<&mut String> = ex.take_extracted::<SelectB>(b_disc).unwrap();
    assert_eq!(strings.len(), 1);
}

/// `take_extracted` for an absent discriminant returns `None`.
#[test]
fn take_extracted_absent_returns_none() {
    let mut data = [E::A(1)];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);
    // Use a type-annotated binding so Rust can infer the output as &mut i32
    let none: Option<Vec<&mut i32>> = ex.take_extracted::<()>(c_disc);
    assert!(none.is_none());
}

/// `take_extracted` preserves the full `'items` lifetime because `TakeFrom`
/// moves `G` into the extractor rather than reborrowing.
#[test]
fn take_extracted_preserves_full_lifetime() {
    let mut data = [E::A(10), E::A(20), E::B("x".into())];
    let a_disc = discriminant(&E::A(0));

    let mut ints: Vec<&mut i32> = {
        let split = split_by_discriminant(&mut data[..], &[a_disc]);
        let mut ex = SplitWithExtractor::new(split, ComplexExtractor);
        ex.take_extracted::<()>(a_disc).unwrap()
        // ex dropped here; ints carry the full 'items lifetime
    };

    *ints[0] = 42;
    *ints[1] = 43;
    drop(ints);
    assert_eq!(data[0], E::A(42));
    assert_eq!(data[1], E::A(43));
}

/// Mutations made through `take_extracted` refs are visible in original data
/// after all borrows are dropped.
#[test]
fn take_extracted_mutations_visible_in_source() {
    let mut data = [E::A(1), E::A(2), E::B("hello".into())];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    {
        let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
        let mut ex = SplitWithExtractor::new(split, ComplexExtractor);
        let mut ints: Vec<&mut i32> = ex.take_extracted::<()>(a_disc).unwrap();
        *ints[0] = 100;
        *ints[1] = 200;
        let mut strings: Vec<&mut String> = ex.take_extracted::<SelectB>(b_disc).unwrap();
        strings[0].push_str(" world");
    }

    assert_eq!(data[0], E::A(100));
    assert_eq!(data[1], E::A(200));
    assert_eq!(data[2], E::B("hello world".into()));
}

// ── TakeFrom trait tests ──────────────────────────────────────────────────────

/// A direct `TakeFrom` impl (not via `ExtractFrom`) works with `take_extracted`.
/// This covers the case where `G` is not `&mut T`, e.g. an owned enum value
/// produced by `map_by_discriminant`.
#[test]
fn take_from_direct_impl_works() {
    // G = E (owned), U = i32 (extracted by value — Copy).
    struct OwnedEExtractor;
    impl TakeFrom<E> for OwnedEExtractor {
        type Output = i32;
        fn take_from(&self, g: E) -> Option<Self::Output> {
            if let E::A(v) = g { Some(v) } else { None }
        }
    }

    let data = vec![E::A(1), E::B("x".into()), E::A(2)];
    let a_disc = discriminant(&E::A(0));

    // owning iterator so G = E
    let split = split_by_discriminant(data.into_iter(), &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, OwnedEExtractor);

    let values: Vec<i32> = ex.take_extracted::<()>(a_disc).unwrap();
    assert_eq!(values, [1, 2]);
    assert!(ex.take_extracted::<()>(a_disc).is_none());
}

/// The blanket impl derives `TakeFrom<&mut T, &mut U>` from `ExtractFrom<T, U>`
/// automatically; verify it compiles and gives correct results via the
/// IpAddr-like foreign-enum pattern.
#[test]
fn take_from_blanket_works_for_foreign_enum() {
    use std::net::{IpAddr, Ipv4Addr};
    struct Ip4Ex;
    impl ExtractFrom<IpAddr> for Ip4Ex {
        type Output<'a> = &'a mut Ipv4Addr;
        fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<Self::Output<'a>> {
            if let IpAddr::V4(v) = t { Some(v) } else { None }
        }
    }

    let mut addrs: Vec<IpAddr> = vec![
        IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
        IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)),
    ];
    let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

    let mut v4s: Vec<&mut Ipv4Addr> = {
        let split = split_by_discriminant(&mut addrs, &[v4_disc]);
        let mut ex = SplitWithExtractor::new(split, Ip4Ex);
        ex.take_extracted::<()>(v4_disc).unwrap()
    };
    *v4s[0] = Ipv4Addr::new(10, 0, 0, 1);
    drop(v4s);
    assert_eq!(addrs[0], IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
}
