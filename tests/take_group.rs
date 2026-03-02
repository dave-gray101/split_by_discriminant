mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, map_by_discriminant};
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
