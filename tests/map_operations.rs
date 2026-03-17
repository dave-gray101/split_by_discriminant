// Tests for map_all and map_others consuming operations on DiscriminantMap,
// plus additional edge cases and corner cases for comprehensive coverage.

mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, map_by_discriminant, DiscriminantMap};
use std::mem::discriminant;

// ── map_all tests ─────────────────────────────────────────────────────────────

#[test]
fn map_all_transforms_each_group() {
    // Test that map_all applies a transformation to each group and consumes self
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Transform each group to its length
    let lengths = split.map_all(|group| group.len());

    assert_eq!(lengths.get(&a_disc), Some(&2));
    assert_eq!(lengths.get(&b_disc), Some(&1));
}

#[test]
fn map_all_collects_transformed_values() {
    // Test that map_all properly collects transformed values into a new DiscriminantMap
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(10), E::A(20), E::A(30)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Transform each i32 to a String representation
    let strings = split.map_all(|group| {
        group
            .into_iter()
            .map(|e| if let E::A(v) = e { format!("Value: {}", v) } else { unreachable!() })
            .collect::<Vec<String>>()
    });

    let group_strs = strings.get(&a_disc).unwrap();
    assert_eq!(group_strs.len(), 3);
    assert_eq!(group_strs[0], "Value: 10");
    assert_eq!(group_strs[1], "Value: 20");
    assert_eq!(group_strs[2], "Value: 30");
}

#[test]
fn map_all_empty_groups_not_in_result() {
    // Test that map_all produces entries only for requested discriminants that had items
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::C];  // No B variant
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let result = split.map_all(|group| group.len());

    // Only A discriminant should have an entry (B was requested but had no items)
    assert_eq!(result.get(&a_disc), Some(&1));
    assert_eq!(result.get(&b_disc), None);
}

#[test]
fn map_all_with_map_by_discriminant() {
    // Test map_all on a result from map_by_discriminant (different G and O types)
    let a_disc = discriminant(&E::A(0));

    let data = [E::A(1), E::A(2), E::C];
    let split: DiscriminantMap<_, String, String> = map_by_discriminant(
        &data[..],
        &[a_disc],
        |e: &E| format!("MATCH:{:?}", e),
        |e: &E| format!("OTHER:{:?}", e),
    );

    // Count items per group
    let counts = split.map_all(|group| group.len());
    assert_eq!(counts.get(&a_disc), Some(&2));
}

#[test]
fn map_by_discriminant_applies_closures() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    // convert matched elements to their debug string, others to a literal
    let split: DiscriminantMap<_, String, String> =
        map_by_discriminant(&mut data[..], &[a_disc, b_disc],
            |r: &mut E| format!("MATCH:{:?}", r),
            |r: &mut E| format!("OTHER:{:?}", r),
        );

    assert_eq!(split.get(a_disc).unwrap(), &[
        String::from("MATCH:A(1)"),
        String::from("MATCH:A(2)"),
    ]);
    assert_eq!(split.get(b_disc).unwrap(), &[String::from("MATCH:B(\"hi\")")]);

    let others = split.into_parts().1;
    assert_eq!(others, vec![String::from("OTHER:C")]);
}

// ── map_others tests ──────────────────────────────────────────────────────────

#[test]
fn map_others_transforms_others_vec() {
    // Test that map_others applies a transformation to the others vector
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::B("hello".into()), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Transform others to their count
    let count: usize = split.map_others(|others| others.len());
    assert_eq!(count, 2);  // B and C
}

#[test]
fn map_others_consumes_others() {
    // Test that map_others takes ownership of the others vec
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::B("x".into()), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Transform others to debug strings
    let debug_strings: Vec<String> = split.map_others(|others| {
        others.into_iter().map(|e| format!("{:?}", e)).collect()
    });

    assert_eq!(debug_strings.len(), 2);
    assert!(debug_strings[0].contains("B"));
    assert!(debug_strings[1].contains("C"));
}

#[test]
fn map_others_empty_when_all_matched() {
    // Test that map_others works correctly when there are no unmatched items
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    let count: usize = split.map_others(|others| others.len());
    assert_eq!(count, 0);
}

#[test]
fn map_others_with_map_by_discriminant() {
    // Test map_others on a map_by_discriminant result where O differs from G
    let a_disc = discriminant(&E::A(0));

    let data = [E::A(1), E::B("x".into()), E::C];
    let split: DiscriminantMap<_, String, String> = map_by_discriminant(
        &data[..],
        &[a_disc],
        |e: &E| format!("MATCH:{:?}", e),
        |e: &E| format!("OTHER:{:?}", e),
    );

    // Count the other items
    let count: usize = split.map_others(|others| others.len());
    assert_eq!(count, 2);
}

// ── Combined map_all and map_others test ──────────────────────────────────────

#[test]
fn map_all_and_map_others_chain() {
    // While you can't call both on the same split (it consumes self),
    // verify the pattern where one is called first
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Map groups first
    let result = split.map_all(|group| group.len());
    assert_eq!(result.get(&a_disc), Some(&2));
    assert_eq!(result.get(&b_disc), Some(&1));
}

// ── into_parts tests ──────────────────────────────────────────────────────────

#[test]
fn into_parts_separates_groups_and_others() {
    // Test that into_parts correctly separates the entries map from the others
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let (groups, others) = split.into_parts();

    // Check groups
    assert_eq!(groups.len(), 2);
    assert!(groups.contains_key(&a_disc));
    assert!(groups.contains_key(&b_disc));
    assert_eq!(groups[&a_disc].len(), 2);
    assert_eq!(groups[&b_disc].len(), 1);

    // Check others
    assert_eq!(others.len(), 1);
    assert!(matches!(others[0], E::C));
}

#[test]
fn into_parts_empty_groups() {
    // Test into_parts when some requested discriminants had no items
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let (groups, others) = split.into_parts();

    // Only A should have an entry
    assert_eq!(groups.len(), 1);
    assert!(groups.contains_key(&a_disc));
    assert!(!groups.contains_key(&b_disc));

    // C is in others
    assert_eq!(others.len(), 1);
}

// ── map_by_discriminant with distinct U and V types ─────────────────────────

/// Calling `map_by_discriminant` with different matched and other output types
/// (U = usize, V = String) exercises the generic fold path where G != O.
#[test]
fn map_by_discriminant_distinct_match_and_other_types() {
    let data = [E::A(3), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    let split: DiscriminantMap<_, usize, String> = map_by_discriminant(
        &data[..],
        &[a_disc],
        |e: &E| if let E::A(v) = e { *v as usize } else { 0 },
        |e: &E| format!("{:?}", e),
    );

    assert_eq!(split.get(a_disc).unwrap(), &[3usize]);
    let (_, others) = split.into_parts();
    assert_eq!(others, vec![String::from("B(\"hi\")"), String::from("C")]);
}

// ── Edge case: owning iterator with map_all and map_others ──────────────────

#[test]
fn map_all_on_owned_values() {
    // Test map_all with owned enum values (from an owning iterator)
    let a_disc = discriminant(&E::A(0));

    let data = vec![E::A(1), E::A(2), E::A(3)];
    let split = split_by_discriminant(data.into_iter(), &[a_disc]);

    // Transform owned E values to i32 values
    let ints = split.map_all(|group| {
        group
            .into_iter()
            .filter_map(|e| if let E::A(v) = e { Some(v) } else { None })
            .collect::<Vec<i32>>()
    });

    assert_eq!(ints.get(&a_disc), Some(&vec![1, 2, 3]));
}

#[test]
fn get_mut_returns_none_for_absent() {
    // Test that get_mut returns None when querying for a discriminant not in the split
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));
    let c_disc = discriminant(&E::C);

    let mut data = [E::A(1)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // b_disc and c_disc were not included in the split
    assert!(split.get_mut(b_disc).is_none());
    assert!(split.get_mut(c_disc).is_none());

    // a_disc is in the split
    assert!(split.get_mut(a_disc).is_some());
}

#[test]
fn get_returns_slice_of_group() {
    // Test that get returns a slice view of the group
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::A(3)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    let group = split.get(a_disc).expect("should have group A");
    assert_eq!(group.len(), 3);
    assert!(group.iter().all(|e| matches!(e, E::A(_))));
    
    // Also verify get returns None for non-existent discriminant
    assert!(split.get(discriminant(&E::B(String::new()))).is_none());
}
