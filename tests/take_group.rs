mod common;
use common::*;

use split_by_discriminant::{
    split_by_discriminant, SplitWithExtractor, map_by_discriminant, TakeFrom, ExtractFrom,
    SplitByDiscriminant,
};
use std::mem::discriminant;

#[test]
fn take_group_returns_owned_vec_and_removes_it() {
    let mut data = [E::A(1), E::A(2), E::B("x".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    // first call succeeds and returns the correct count
    let group_a: Vec<&mut E> = split.take_group(a_disc).unwrap();
    assert_eq!(group_a.len(), 2);

    // second call for the same discriminant returns None (already removed)
    assert!(split.take_group(a_disc).is_none());

    // other groups are unaffected
    let group_b: Vec<&mut E> = split.take_group(b_disc).unwrap();
    assert_eq!(group_b.len(), 1);
}

#[test]
fn take_group_returns_none_for_absent_discriminant() {
    let mut data = [E::A(1)];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    // C was never a requested discriminant
    assert!(split.take_group(c_disc).is_none());
}

#[test]
fn take_group_preserves_full_lifetime_on_mut_refs() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into())];
    let a_disc = discriminant(&E::A(0));

    // 'items is the lifetime of data's borrow; the split is shorter-lived
    let group: Vec<&mut E> = {
        let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
        // take_group moves the Vec out, so 'items survives the split's drop
        split.take_group(a_disc).unwrap()
        // `split` is dropped here; `group` continues to hold &mut E with 'items
    };

    // The group outlives the split itself — and we can still mutate through it
    for item in group {
        if let E::A(v) = item { *v += 98; }
    }

    assert_eq!(data[0], E::A(99));
    assert_eq!(data[1], E::A(100));
}

#[test]
fn take_group_then_extract_manual_preserves_lifetime() {
    let mut data = [E::A(10), E::A(20), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut ints: Vec<&mut i32> = {
        let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
        let group: Vec<&mut E> = split.take_group(a_disc).unwrap();
        // no split borrow alive past this point; ints carry the full 'items
        group
            .into_iter()
            .filter_map(|e| if let E::A(v) = e { Some(v) } else { None })
            .collect()
    };

    *ints[0] = 55;
    *ints[1] = 66;
    drop(ints);

    assert_eq!(data[0], E::A(55));
    assert_eq!(data[1], E::A(66));
}

#[test]
fn take_group_on_split_with_extractor() {
    let mut data = [E::A(3), E::A(4), E::B("z".into())];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut extractor = SplitWithExtractor::new(split, EExtractor);

    let group_a: Vec<&mut E> = extractor.take_group(a_disc).unwrap();
    assert_eq!(group_a.len(), 2);

    // second call returns None
    assert!(extractor.take_group(a_disc).is_none());

    let group_b: Vec<&mut E> = extractor.take_group(b_disc).unwrap();
    assert_eq!(group_b.len(), 1);
}

#[test]
fn take_group_on_extractor_preserves_full_lifetime() {
    let mut data = [E::A(7), E::A(8)];
    let a_disc = discriminant(&E::A(0));

    let mut ints: Vec<&mut i32> = {
        let split = split_by_discriminant(&mut data[..], &[a_disc]);
        let mut extractor = SplitWithExtractor::new(split, EExtractor);
        let group: Vec<&mut E> = extractor.take_group(a_disc).unwrap();
        // extractor dropped here; ints still carry 'items
        group
            .into_iter()
            .filter_map(|e| if let E::A(v) = e { Some(v) } else { None })
            .collect()
    };

    *ints[0] = 77;
    *ints[1] = 88;
    drop(ints);

    assert_eq!(data[0], E::A(77));
    assert_eq!(data[1], E::A(88));
}

#[test]
fn take_group_on_map_by_discriminant_result() {
    let data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    use split_by_discriminant::SplitByDiscriminant;

    let mut split: SplitByDiscriminant<_, String, String> =
        map_by_discriminant(&data[..], &[a_disc, b_disc],
            |r: &E| format!("MATCH:{:?}", r),
            |r: &E| format!("OTHER:{:?}", r),
        );

    let group_a: Vec<String> = split.take_group(a_disc).unwrap();
    assert_eq!(group_a, vec!["MATCH:A(1)", "MATCH:A(2)"]);

    // removed — second call returns None
    assert!(split.take_group(a_disc).is_none());

    let group_b: Vec<String> = split.take_group(b_disc).unwrap();
    assert_eq!(group_b, vec!["MATCH:B(\"hi\")"]);

    // others are still intact
    let (_, others) = split.into_parts();
    assert_eq!(others, vec!["OTHER:C"]);
}

#[test]
fn take_group_on_owning_iterator() {
    let data = vec![E::A(1), E::B("x".into()), E::A(2)];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(data.into_iter(), &[a_disc]);
    let group: Vec<E> = split.take_group(a_disc).unwrap();
    assert_eq!(group.len(), 2);
    assert_eq!(group[0], E::A(1));
    assert_eq!(group[1], E::A(2));
}

// ── take_group_mapped tests ───────────────────────────────────────────────────

/// `take_group_mapped` transforms each element via a closure and removes the group.
#[test]
fn take_group_mapped_transforms_and_removes() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    let labels: Vec<String> = split
        .take_group_mapped(a_disc, |e| format!("{:?}", e))
        .unwrap();
    assert_eq!(labels, ["A(1)", "A(2)"]);

    // second call returns None — group consumed
    assert!(split.take_group_mapped(a_disc, |e| format!("{:?}", e)).is_none());

    // other group unaffected
    let b_labels: Vec<String> = split
        .take_group_mapped(b_disc, |e| format!("{:?}", e))
        .unwrap();
    assert_eq!(b_labels, ["B(\"hi\")"]);
}

/// `take_group_mapped` with a `map_by_discriminant` result where `G` is `String`.
#[test]
fn take_group_mapped_on_map_result() {
    let data = [E::A(1), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut split: SplitByDiscriminant<_, String, String> = map_by_discriminant(
        &data[..],
        &[a_disc],
        |e: &E| format!("M:{:?}", e),
        |e: &E| format!("O:{:?}", e),
    );

    // map each String to its length — "M:A(1)".len() == 6
    let lens: Vec<usize> = split.take_group_mapped(a_disc, |s| s.len()).unwrap();
    assert_eq!(lens, [6, 6]);
}

/// `take_group_mapped` via `SplitWithExtractor` delegates to the inner split.
#[test]
fn take_group_mapped_via_extractor_wrapper() {
    let mut data = [E::A(10), E::A(20), E::B("z".into())];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);

    let labels: Vec<String> = ex
        .take_group_mapped(a_disc, |e| format!("{:?}", e))
        .unwrap();
    assert_eq!(labels, ["A(10)", "A(20)"]);
    assert!(ex.take_group_mapped(a_disc, |e| format!("{:?}", e)).is_none());
}

/// `take_group_mapped` preserves the full lifetime when `G = &'items mut T`:
/// elements are consumed by value so the returned `U` (here `&mut i32`) derives
/// its lifetime from `'items`, not from a short reborrow.
#[test]
fn take_group_mapped_preserves_lifetime() {
    let mut data = [E::A(1), E::A(2), E::B("x".into())];
    let a_disc = discriminant(&E::A(0));

    // The returned Vec outlives the split itself.
    let mut ints: Vec<&mut i32> = {
        let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
        split
            .take_group_mapped(a_disc, |e| if let E::A(v) = e { v } else { panic!() })
            .unwrap()
    };
    *ints[0] = 77;
    *ints[1] = 88;
    drop(ints);
    assert_eq!(data[0], E::A(77));
    assert_eq!(data[1], E::A(88));
}

// ── take_group_with tests ─────────────────────────────────────────────────────

/// Basic: `take_group_with` filter-maps elements and removes the group.
#[test]
fn take_group_with_filter_maps_and_removes() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    // extract inner i32 refs from A variants — B variant in the same group would be skipped
    let ints: Vec<&mut i32> = split
        .take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);

    // group is gone
    assert!(split
        .take_group_with::<i32, _>(a_disc, |_| None)
        .is_none());

    // other group unaffected
    let strings: Vec<&mut String> = split
        .take_group_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strings.len(), 1);
}

/// `take_group_with` returns `None` for an absent discriminant.
#[test]
fn take_group_with_absent_returns_none() {
    let mut data = [E::A(1)];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    assert!(split
        .take_group_with::<&mut i32, _>(c_disc, |_| None)
        .is_none());
}

/// Core lifetime test: the closure receives `G` by value (moved), so the
/// returned references carry the full `'items` lifetime and the Vec can
/// outlive the split.
#[test]
fn take_group_with_preserves_full_lifetime() {
    let mut data = [E::A(1), E::A(2), E::B("x".into())];
    let a_disc = discriminant(&E::A(0));

    let mut ints: Vec<&mut i32> = {
        let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
        split
            .take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
            .unwrap()
        // split dropped here; ints still carry 'items
    };

    *ints[0] = 55;
    *ints[1] = 66;
    drop(ints);
    assert_eq!(data[0], E::A(55));
    assert_eq!(data[1], E::A(66));
}

/// `take_group_with` on an owning iterator (`G = E`) — closure receives
/// owned values and may return owned results.
#[test]
fn take_group_with_on_owning_iterator() {
    let data = vec![E::A(1), E::B("x".into()), E::A(2)];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(data.into_iter(), &[a_disc]);
    let values: Vec<i32> = split
        .take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(values, [1, 2]);
    assert!(split.take_group_with::<i32, _>(a_disc, |_| None).is_none());
}

/// `take_group_with` on a `map_by_discriminant` result where `G = String`.
#[test]
fn take_group_with_on_map_result() {
    let data = [E::A(1), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut split: SplitByDiscriminant<_, String, String> = map_by_discriminant(
        &data[..],
        &[a_disc],
        |e: &E| format!("{:?}", e),
        |e: &E| format!("{:?}", e),
    );

    // keep only strings that start with 'A'
    let matched: Vec<String> = split
        .take_group_with(a_disc, |s| if s.starts_with('A') { Some(s) } else { None })
        .unwrap();
    assert_eq!(matched, ["A(1)", "A(2)"]);
}

/// `take_group_with` forwarded through `SplitWithExtractor`.
#[test]
fn take_group_with_via_extractor_wrapper() {
    let mut data = [E::A(3), E::A(4), E::B("z".into())];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);

    let ints: Vec<&mut i32> = ex
        .take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    assert!(ex
        .take_group_with::<i32, _>(a_disc, |_| None)
        .is_none());
}

/// `take_group_with` via `SplitWithExtractor` preserves the full `'items`
/// lifetime — same guarantee as on `SplitByDiscriminant` directly.
#[test]
fn take_group_with_via_extractor_preserves_lifetime() {
    let mut data = [E::A(7), E::A(8)];
    let a_disc = discriminant(&E::A(0));

    let mut ints: Vec<&mut i32> = {
        let split = split_by_discriminant(&mut data[..], &[a_disc]);
        let mut ex = SplitWithExtractor::new(split, EExtractor);
        ex.take_group_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
            .unwrap()
    };
    *ints[0] = 99;
    *ints[1] = 100;
    drop(ints);
    assert_eq!(data[0], E::A(99));
    assert_eq!(data[1], E::A(100));
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
    let mut ex = SplitWithExtractor::new(split, EExtractor);

    let ints: Vec<&mut i32> = ex.take_extracted(a_disc).unwrap();
    assert_eq!(ints.len(), 2);

    // group consumed — second call returns None
    assert!(ex.take_extracted::<&mut i32>(a_disc).is_none());

    // other group still accessible
    let strings: Vec<&mut String> = ex.take_extracted(b_disc).unwrap();
    assert_eq!(strings.len(), 1);
}

/// `take_extracted` for an absent discriminant returns `None`.
#[test]
fn take_extracted_absent_returns_none() {
    let mut data = [E::A(1)];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);
    // Use a type-annotated binding so Rust can infer U = &mut i32
    let none: Option<Vec<&mut i32>> = ex.take_extracted(c_disc);
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
        let mut ex = SplitWithExtractor::new(split, EExtractor);
        ex.take_extracted(a_disc).unwrap()
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
        let mut ex = SplitWithExtractor::new(split, EExtractor);
        let mut ints: Vec<&mut i32> = ex.take_extracted(a_disc).unwrap();
        *ints[0] = 100;
        *ints[1] = 200;
        let mut strings: Vec<&mut String> = ex.take_extracted(b_disc).unwrap();
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
    impl TakeFrom<E, i32> for OwnedEExtractor {
        fn take_from(&self, g: E) -> Option<i32> {
            if let E::A(v) = g { Some(v) } else { None }
        }
    }

    let data = vec![E::A(1), E::B("x".into()), E::A(2)];
    let a_disc = discriminant(&E::A(0));

    // owning iterator so G = E
    let split = split_by_discriminant(data.into_iter(), &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, OwnedEExtractor);

    let values: Vec<i32> = ex.take_extracted(a_disc).unwrap();
    assert_eq!(values, [1, 2]);
    assert!(ex.take_extracted::<i32>(a_disc).is_none());
}

/// The blanket impl derives `TakeFrom<&mut T, &mut U>` from `ExtractFrom<T, U>`
/// automatically; verify it compiles and gives correct results via the
/// IpAddr-like foreign-enum pattern.
#[test]
fn take_from_blanket_works_for_foreign_enum() {
    use std::net::{IpAddr, Ipv4Addr};
    struct Ip4Ex;
    impl ExtractFrom<IpAddr, Ipv4Addr> for Ip4Ex {
        fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv4Addr> {
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
        ex.take_extracted(v4_disc).unwrap()
    };
    *v4s[0] = Ipv4Addr::new(10, 0, 0, 1);
    drop(v4s);
    assert_eq!(addrs[0], IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
}

// ── take_others ──────────────────────────────────────────────────────────────

#[test]
fn take_others_returns_unmatched_items() {
    let mut data = [E::A(1), E::A(2), E::B("x".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    let others: Vec<&mut E> = split.take_others();
    // B and C are unmatched
    assert_eq!(others.len(), 2);
}

#[test]
fn take_others_second_call_returns_empty() {
    let mut data = [E::A(1), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    let first: Vec<&mut E> = split.take_others();
    assert_eq!(first.len(), 1);

    // unlike take_group, a second call returns an empty vec rather than None
    let second: Vec<&mut E> = split.take_others();
    assert!(second.is_empty());
}

#[test]
fn take_others_groups_intact_after_call() {
    let mut data = [E::A(1), E::A(2), E::B("y".into())];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(&mut data, &[a_disc]);

    // taking others should not disturb any group
    let _others: Vec<&mut E> = split.take_others();
    let group: Vec<&mut E> = split.take_group(a_disc).unwrap();
    assert_eq!(group.len(), 2);
}

#[test]
fn take_others_preserves_full_lifetime() {
    let mut data = [E::A(1), E::C, E::B("z".into())];
    let a_disc = discriminant(&E::A(0));

    // others outlives the split — same lifetime guarantee as take_group
    let others: Vec<&mut E> = {
        let mut split = split_by_discriminant(&mut data[..], &[a_disc]);
        split.take_others()
    };
    assert_eq!(others.len(), 2);
}

#[test]
fn take_others_on_split_with_extractor() {
    let mut data = [E::A(1), E::B("w".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);
    let others: Vec<&mut E> = ex.take_others();
    assert_eq!(others.len(), 2); // B and C
}

// ── others() borrow ────────────────────────────────────────────────────────────────────────────

#[test]
fn others_returns_slice_of_unmatched() {
    let mut data = [E::A(1), E::B("x".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    // &self — no mutable borrow required
    assert_eq!(split.others().len(), 2); // B and C
}

#[test]
fn others_empty_when_all_matched() {
    let mut data = [E::A(1), E::A(2)];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    assert!(split.others().is_empty());
}

#[test]
fn others_empty_after_take_others() {
    let mut data = [E::A(1), E::C];
    let a_disc = discriminant(&E::A(0));

    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    let _ = split.take_others();
    // take_others drains the vec; borrow slice is now empty
    assert!(split.others().is_empty());
}

#[test]
fn others_does_not_require_mut_borrow() {
    // Compile-time proof: group() requires &mut self but others() only needs &self,
    // so both can be called on an immutable binding.
    let data = [E::A(1), E::B("y".into())];
    let a_disc = discriminant(&E::A(0));

    // immutable binding — others() still compiles
    let split = split_by_discriminant(&data[..], &[a_disc]);
    assert_eq!(split.others().len(), 1);
}

#[test]
fn others_on_split_with_extractor() {
    let mut data = [E::A(1), E::B("z".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let ex = SplitWithExtractor::new(split, EExtractor);
    assert_eq!(ex.others().len(), 2); // B and C
}
