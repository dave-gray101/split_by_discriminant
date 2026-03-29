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

// ── get_multiple tests ─────────────────────────────────────────────────────────

#[test]
fn get_multiple_retrieves_multiple_groups() {
    // Test that get_multiple returns all requested discriminants that are present
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let result = split.get_multiple(&[a_disc, b_disc]);
    
    // Both discriminants should be present
    assert_eq!(result.len(), 2);
    assert!(result.contains_key(&a_disc));
    assert!(result.contains_key(&b_disc));
    
    // Verify the contents
    assert_eq!(result.get(&a_disc).unwrap().len(), 2);
    assert_eq!(result.get(&b_disc).unwrap().len(), 2);
}

#[test]
fn get_multiple_partial_match() {
    // Test that get_multiple only returns discriminants that exist in the split
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));
    let c_disc = discriminant(&E::C);

    let mut data = [E::A(1), E::B("hi".into()), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Request A, B, and C even though only A and B were split on
    let result = split.get_multiple(&[a_disc, b_disc, c_disc]);
    
    // Only A and B should be present
    assert_eq!(result.len(), 2);
    assert!(result.contains_key(&a_disc));
    assert!(result.contains_key(&b_disc));
    assert!(!result.contains_key(&c_disc));
}

#[test]
fn get_multiple_empty_ids() {
    // Test that get_multiple returns an empty map when given an empty ids slice
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    let result = split.get_multiple(&[]);
    assert_eq!(result.len(), 0);
}

#[test]
fn get_multiple_with_duplicates_in_ids() {
    // Test that duplicate discriminants in the ids slice are handled correctly
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Request with duplicates: [A, B, A, A] — should return A and B only once
    let result = split.get_multiple(&[a_disc, b_disc, a_disc, a_disc]);
    
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(&a_disc).unwrap().len(), 2);
    assert_eq!(result.get(&b_disc).unwrap().len(), 1);
}

#[test]
fn get_multiple_returns_correct_references() {
    // Test that get_multiple returns references with the correct lifetime
    // (tied to the split's borrow, not to the input ids slice)
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::A(2), E::B("hi".into())];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    {
        let ids = [a_disc, b_disc];  // Create a temporary ids array
        let result = split.get_multiple(&ids);
        
        // result is now tied to split's borrow, not ids
        assert_eq!(result.len(), 2);
    }  // ids goes out of scope here

    // The fact that we can still query means the references are correct
    let result2 = split.get_multiple(&[a_disc]);
    assert_eq!(result2.len(), 1);
}

#[test]
fn get_multiple_single_element() {
    // Test get_multiple with just one discriminant in the ids slice
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    let result = split.get_multiple(&[a_disc]);
    
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(&a_disc).unwrap().len(), 2);
}

// ── for_each_group_mut tests ─────────────────────────────────────────────────

#[test]
fn for_each_group_mut_retrieves_and_allows_mutation() {
    // Test that for_each_group_mut visits all requested groups and allows mutation
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let mut a_seen = false;
    let mut b_seen = false;
    split.for_each_group_mut(&[a_disc, b_disc], |disc, mut group| {
        if disc == a_disc {
            a_seen = true;
            for item in group.iter_mut() {
                if let E::A(v) = item { *v += 10; }
            }
        } else if disc == b_disc {
            b_seen = true;
        }
    });

    // Both discriminants should have been visited
    assert!(a_seen);
    assert!(b_seen);

    // Verify the mutations persisted
    assert_eq!(*split.get(a_disc).unwrap()[0], E::A(11));
    assert_eq!(*split.get(a_disc).unwrap()[1], E::A(12));
}

#[test]
fn for_each_group_mut_partial_match() {
    // Test that for_each_group_mut only visits discriminants present in the split
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));
    let c_disc = discriminant(&E::C);

    let mut data = [E::A(5), E::B("test".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let mut seen = std::collections::HashSet::new();
    // Request A, B, and C — only A and B should be visited
    split.for_each_group_mut(&[a_disc, b_disc, c_disc], |disc, _| {
        seen.insert(disc);
    });

    assert_eq!(seen.len(), 2);
    assert!(seen.contains(&a_disc));
    assert!(seen.contains(&b_disc));
    assert!(!seen.contains(&c_disc));
}

#[test]
fn for_each_group_mut_empty_ids() {
    // Test that for_each_group_mut with an empty slice visits no groups
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let mut count = 0usize;
    split.for_each_group_mut(&[], |_, _| { count += 1; });
    assert_eq!(count, 0);
}

#[test]
fn for_each_group_mut_with_duplicates() {
    // Test that duplicate discriminants in ids are deduplicated
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(10), E::B("x".into()), E::A(20)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let mut groups_visited = 0usize;
    let mut a_len = 0usize;
    split.for_each_group_mut(&[a_disc, b_disc, a_disc], |disc, group| {
        groups_visited += 1;
        if disc == a_disc { a_len = group.len(); }
    });

    // Deduplicated: A and B each visited once
    assert_eq!(groups_visited, 2);
    assert_eq!(a_len, 2);
}

#[test]
fn for_each_group_mut_string_mutation() {
    // Test mutating String elements through for_each_group_mut
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hello".into()), E::B("world".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    split.for_each_group_mut(&[b_disc], |_, mut group| {
        for item in group.iter_mut() {
            if let E::B(s) = item { s.push('!'); }
        }
    });

    // Verify mutations
    let b_items = split.get(b_disc).unwrap();
    assert!(matches!(&b_items[0], E::B(s) if s == "hello!"));
    assert!(matches!(&b_items[1], E::B(s) if s == "world!"));
}

#[test]
fn for_each_group_mut_callback_lifetime() {
    // Test that the callback borrows split for exactly the duration of each call
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let mut count = 0usize;
    {
        let ids = [a_disc];
        split.for_each_group_mut(&ids, |_, group| { count = group.len(); });
    } // ids goes out of scope here

    // count captured the group length; split is still usable
    assert_eq!(count, 2);
    let mut count2 = 0usize;
    split.for_each_group_mut(&[a_disc], |_, group| { count2 = group.len(); });
    assert_eq!(count2, 2);
}

// ── GroupMut direct-method tests ──────────────────────────────────────────────

#[test]
fn group_mut_len_and_is_empty() {
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::A(2), E::A(3)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let group = split.get_mut(a_disc).unwrap();
    assert_eq!(group.len(), 3);
    assert!(!group.is_empty());

    // b_disc was requested but no items matched — group is absent entirely
    assert!(split.get_mut(b_disc).is_none());
}

#[test]
fn group_mut_as_slice_and_index() {
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(10), E::A(20)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let group = split.get_mut(a_disc).unwrap();

    // as_slice() gives a shared view
    let slice = group.as_slice();
    assert_eq!(slice.len(), 2);

    // Index gives the same element without IndexMut
    assert!(matches!(group[0], E::A(10)));
    assert!(matches!(group[1], E::A(20)));
}

#[test]
fn group_mut_shared_ref_into_iterator() {
    // for item in &group should iterate shared refs without consuming the GroupMut
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::A(3)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let group = split.get_mut(a_disc).unwrap();

    let vals: Vec<i32> = (&group)
        .into_iter()
        .filter_map(|e| if let E::A(v) = e { Some(*v) } else { None })
        .collect();
    assert_eq!(vals, vec![1, 2, 3]);

    // group is still usable after shared ref iteration
    assert_eq!(group.len(), 3);
}

#[test]
fn group_mut_sort_by_and_reverse() {
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(3), E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    {
        let mut group = split.get_mut(a_disc).unwrap();

        // sort_by: ascending by value
        group.sort_by(|a, b| {
            let E::A(va) = a else { unreachable!() };
            let E::A(vb) = b else { unreachable!() };
            va.cmp(vb)
        });

        let sorted: Vec<i32> = group.iter()
            .filter_map(|e| if let E::A(v) = e { Some(*v) } else { None })
            .collect();
        assert_eq!(sorted, vec![1, 2, 3]);

        // reverse in place
        group.reverse();
    }

    let after: Vec<i32> = split.get(a_disc).unwrap().iter()
        .filter_map(|e| if let E::A(v) = e { Some(*v) } else { None })
        .collect();
    assert_eq!(after, vec![3, 2, 1]);
}

#[test]
fn group_mut_sort_unstable_by() {
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(5), E::A(2), E::A(8), E::A(1)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    {
        let mut group = split.get_mut(a_disc).unwrap();
        group.sort_unstable_by(|a, b| {
            let E::A(va) = a else { unreachable!() };
            let E::A(vb) = b else { unreachable!() };
            va.cmp(vb)
        });
    }

    let sorted: Vec<i32> = split.get(a_disc).unwrap().iter()
        .filter_map(|e| if let E::A(v) = e { Some(*v) } else { None })
        .collect();
    assert_eq!(sorted, vec![1, 2, 5, 8]);
}

// ── remove_multiple tests ─────────────────────────────────────────────────────

#[test]
fn remove_multiple_claims_ownership_of_groups() {
    // Test that remove_multiple returns owned groups for the requested discriminants
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let removed = split.remove_multiple(&[a_disc, b_disc]);

    // Removed should contain both groups
    assert_eq!(removed.len(), 2);
    assert_eq!(removed.get(&a_disc).unwrap().len(), 2);
    assert_eq!(removed.get(&b_disc).unwrap().len(), 2);

    // Original split should now have empty maps for these discriminants
    assert!(split.get(a_disc).is_none());
    assert!(split.get(b_disc).is_none());
}

#[test]
fn remove_multiple_partial_match() {
    // Test that remove_multiple only removes discriminants that exist
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));
    let c_disc = discriminant(&E::C);

    let mut data = [E::A(1), E::B("hi".into()), E::C];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Request removal of A, B, and C even though C was never split on
    let removed = split.remove_multiple(&[a_disc, b_disc, c_disc]);

    // Only A and B should be in the result
    assert_eq!(removed.len(), 2);
    assert!(removed.contains_key(&a_disc));
    assert!(removed.contains_key(&b_disc));
    assert!(!removed.contains_key(&c_disc));
}

#[test]
fn remove_multiple_empty_ids() {
    // Test that remove_multiple with empty ids returns an empty map
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let removed = split.remove_multiple(&[]);
    assert_eq!(removed.len(), 0);

    // Original split should still have the group
    assert_eq!(split.get(a_disc).unwrap().len(), 2);
}

#[test]
fn remove_multiple_with_duplicates() {
    // Test that duplicate ids in the slice are handled correctly
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Request removal twice with duplicate
    let removed = split.remove_multiple(&[a_disc, a_disc]);

    // Should only have one entry (duplicates collapse)
    assert_eq!(removed.len(), 1);
    assert_eq!(removed.get(&a_disc).unwrap().len(), 2);
}

// ── remove_multiple_mapped tests ──────────────────────────────────────────────

#[test]
fn remove_multiple_mapped_transforms_all_groups() {
    // Test that remove_multiple_mapped removes and transforms each group
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Transform to lengths
    let transformed = split.remove_multiple_mapped(&[a_disc, b_disc], |e| match e {
        E::A(v) => *v as usize,
        E::B(s) => s.len(),
        E::C => 0,
    });

    assert_eq!(transformed.get(&a_disc).unwrap(), &[1usize, 2usize]);
    assert_eq!(transformed.get(&b_disc).unwrap(), &[2usize, 3usize]); // "hi" and "bye"
}

#[test]
fn remove_multiple_mapped_partial_match() {
    // Test that only present discriminants are transformed
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(5), E::A(10)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Request transformation for A and B, but only A exists
    let transformed = split.remove_multiple_mapped(&[a_disc, b_disc], |e| match e {
        E::A(v) => *v * 2,
        _ => 0,
    });

    assert_eq!(transformed.len(), 1);
    assert_eq!(transformed.get(&a_disc).unwrap(), &[10, 20]);
    assert!(!transformed.contains_key(&b_disc));
}

// ── remove_multiple_with tests ────────────────────────────────────────────────

#[test]
fn remove_multiple_with_filters_and_transforms() {
    // Test that remove_multiple_with filters (Some/None) and transforms each group
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Filter: A values > 1, B strings longer than 2 chars
    let filtered = split.remove_multiple_with(&[a_disc, b_disc], |e| match e {
        E::A(v) if *v > 1 => Some(*v as usize),
        E::B(s) if s.len() > 2 => Some(s.len()),
        _ => None,
    });

    assert_eq!(filtered.get(&a_disc).unwrap(), &[2usize]);  // only 2 > 1
    assert_eq!(filtered.get(&b_disc).unwrap(), &[3usize]); // only "bye" > 2 chars
}

#[test]
fn remove_multiple_with_all_filtered_out() {
    // Test when all items in a group are filtered out
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Filter for values > 10 (none match)
    let filtered = split.remove_multiple_with(&[a_disc], |e| match e {
        E::A(v) if *v > 10 => Some(*v),
        _ => None,
    });

    // Map contains the key but with an empty vec
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered.get(&a_disc).unwrap().len(), 0);
}

#[test]
fn remove_multiple_with_partial_match() {
    // Test remove_multiple_with with some discriminants not in the split
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let filtered = split.remove_multiple_with(&[a_disc, b_disc], |e| match e {
        E::A(v) => Some(*v),
        _ => None,
    });

    assert_eq!(filtered.len(), 1);
    assert!(filtered.contains_key(&a_disc));
    assert!(!filtered.contains_key(&b_disc));
}

// ── extract_multiple_with tests ───────────────────────────────────────────────

#[test]
fn extract_multiple_with_extracts_from_multiple_groups() {
    // Test that extract_multiple_with extracts from all requested groups
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::B("bye".into())];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    let extracted = split.extract_multiple_with(&[a_disc, b_disc], |e| match e {
        E::A(v) => Some(*v as usize),
        E::B(s) => Some(s.len()),
        _ => None,
    });

    assert_eq!(extracted.len(), 2);
    assert_eq!(extracted.get(&a_disc).unwrap(), &[1usize, 2usize]);
    assert_eq!(extracted.get(&b_disc).unwrap(), &[2usize, 3usize]);
}

#[test]
fn extract_multiple_with_partial_match() {
    // Test extract_multiple_with with some discriminants not in the split
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(5), E::A(10)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let extracted = split.extract_multiple_with(&[a_disc, b_disc], |e| match e {
        E::A(v) => Some(*v * 2),
        _ => None,
    });

    assert_eq!(extracted.len(), 1);
    assert_eq!(extracted.get(&a_disc).unwrap(), &[10, 20]);
    assert!(!extracted.contains_key(&b_disc));
}

#[test]
fn extract_multiple_with_filtering() {
    // Test extract_multiple_with with filtering (Some/None)
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::A(3), E::A(4)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Extract only even values
    let extracted = split.extract_multiple_with(&[a_disc], |e| match e {
        E::A(v) if *v % 2 == 0 => Some(*v),
        _ => None,
    });

    assert_eq!(extracted.get(&a_disc).unwrap(), &[2, 4]);
}

#[test]
fn extract_multiple_with_empty_ids() {
    // Test extract_multiple_with with empty ids slice
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let extracted = split.extract_multiple_with(&[], |e| match e {
        E::A(v) => Some(*v),
        _ => None,
    });

    assert_eq!(extracted.len(), 0);

    // Original split should still have the group (extract doesn't remove)
    assert_eq!(split.get(a_disc).unwrap().len(), 2);
}

#[test]
fn extract_multiple_with_doesnt_remove() {
    // Test that extract_multiple_with doesn't remove the groups
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    let _extracted = split.extract_multiple_with(&[a_disc], |e| match e {
        E::A(v) => Some(*v),
        _ => None,
    });

    // Groups should still be accessible
    assert_eq!(split.get(a_disc).unwrap().len(), 2);

    // Can extract again
    let extracted2 = split.extract_multiple_with(&[a_disc], |e| match e {
        E::A(v) => Some(*v * 10),
        _ => None,
    });

    assert_eq!(extracted2.get(&a_disc).unwrap(), &[10, 20]);
}
// ── others_mut test ───────────────────────────────────────────────────────────

#[test]
fn others_mut_gives_mutable_access() {
    let a_disc = discriminant(&E::A(0));
    let mut data = [E::A(1), E::C, E::A(2), E::C];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Confirm others_mut compiles and yields the right count
    assert_eq!(split.others_mut().len(), 2);
    for item in split.others_mut() {
        assert!(matches!(item, E::C));
    }
}

// ── extract_with (single group) test ─────────────────────────────────────────

#[test]
fn extract_with_extracts_owned_values() {
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);
    let mut data = [E::A(3), E::A(7), E::A(4)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Extract only values > 4
    let extracted = split
        .extract_with(a_disc, |e| {
            if let E::A(v) = e { if *v > 4 { Some(*v) } else { None } } else { None }
        })
        .unwrap();
    assert_eq!(extracted, vec![7]);

    // Group is still present (extract_with does not remove)
    assert_eq!(split.get(a_disc).unwrap().len(), 3);

    // Absent discriminant returns None
    assert!(split.extract_with(c_disc, |_| Some(0u8)).is_none());
}

// ── &mut GroupMut into_iter test ──────────────────────────────────────────────

#[test]
fn group_mut_mut_ref_into_iterator() {
    let a_disc = discriminant(&E::A(0));
    let mut data = [E::A(1), E::A(2), E::A(3)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    {
        let mut group = split.get_mut(a_disc).unwrap();
        // IntoIterator for &mut GroupMut<G> — yields &mut G
        for item in &mut group {
            if let E::A(v) = item { *v += 10; }
        }
    }

    let vals: Vec<i32> = split.get(a_disc).unwrap().iter()
        .filter_map(|e| if let E::A(v) = e { Some(*v) } else { None })
        .collect();
    assert_eq!(vals, vec![11, 12, 13]);
}