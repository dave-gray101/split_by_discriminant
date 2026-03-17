mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, DiscriminantMap};
use std::mem::discriminant;

#[test]
fn vec_and_slice_are_supported() {
    let mut vec = vec![E::A(7), E::B("z".into()), E::A(8)];
    let a_disc = discriminant(&E::A(0));

    // Vec via &mut Vec<T>
    let splitv = split_by_discriminant(&mut vec, &[a_disc]);
    assert_eq!(splitv.get(a_disc).unwrap().len(), 2);

    // mutable slice as well
    let mut array = [E::A(3), E::C];
    let splits = split_by_discriminant(&mut array[..], &[a_disc]);
    assert_eq!(splits.get(a_disc).unwrap().len(), 1);

    // immutable slice works with the same generic function
    let array2 = [E::A(10), E::B("foo".into())];
    let splitr: DiscriminantMap<_, &E> =
        split_by_discriminant(&array2[..], &[a_disc]);
    assert_eq!(splitr.get(a_disc).unwrap().len(), 1);
}

#[test]
fn get_mut_and_remove_with_work_on_mutable_borrows() {
    let mut data = [E::A(4), E::B("x".into())];
    let a_disc = discriminant(&E::A(0));

    // mutable iterator yields &mut E — get_mut and remove_with are available
    let mut s1 = split_by_discriminant(&mut data[..], &[a_disc]);
    let _ints: Vec<&mut i32> = s1
        .remove_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();

    let data2 = [E::A(5), E::C];
    // immutable iterator yields &E
    let s2: DiscriminantMap<_, &E> =
        split_by_discriminant(&data2[..], &[a_disc]);
    assert_eq!(s2.get(a_disc).unwrap().len(), 1);
}

#[test]
fn owning_iterator_is_supported() {
    let a_disc = discriminant(&E::A(0));
    let data = vec![E::A(1), E::B("x".into()), E::A(2)];

    // we consume the vec, taking ownership of the items
    let split = split_by_discriminant(data.into_iter(), &[a_disc]);
    let (groups, others) = split.into_parts();
    assert_eq!(groups.get(&a_disc).unwrap().len(), 2);
    assert_eq!(others.len(), 1);
}

#[test]
fn absent_discriminant_returns_none() {
    let mut data = [E::A(5), E::B("foo".into())];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    // only split on A; C is absent — get and SplitWithExtractor::extract
    // must both return None
    let split = split_by_discriminant(&mut data, &[a_disc]);
    assert!(split.get(c_disc).is_none());
}

#[test]
fn remove_with_no_trait_impl_required() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    // closure plays the role of what user_helper would export as a free fn
    // remove_with moves elements out, so no trait impl is required
    let mut ints: Vec<&mut i32> = split
        .remove_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    *ints[0] = 99;

    let strings: Vec<&mut String> = split
        .remove_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strings.len(), 1);

    // nonexistent discriminant still returns None
    let c_disc = discriminant(&E::C);
    assert!(split
        .remove_with::<&mut i32, _>(c_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .is_none());

    drop(split);
    assert_eq!(data[0], E::A(99));
}

#[test]
fn duplicate_discriminants_in_kinds_are_ignored() {
    let mut data = [E::A(1), E::B("x".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    // Pass duplicates in the kinds slice — they should be deduplicated internally
    let split = split_by_discriminant(&mut data, &[a_disc, b_disc, a_disc, a_disc, b_disc]);

    // Should still have both groups with correct content
    assert_eq!(split.get(a_disc).unwrap().len(), 2);
    assert_eq!(split.get(b_disc).unwrap().len(), 1);
    assert_eq!(split.others().len(), 1); // C is in others
}
