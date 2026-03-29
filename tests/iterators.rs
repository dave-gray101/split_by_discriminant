// Tests for IntoIterator implementations on DiscriminantMap and SplitWithExtractor,
// plus the take_simple and take_extracted consuming extraction methods.

mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;

// ── DiscriminantMap IntoIterator tests ────────────────────────────────────────

#[test]
fn discriminant_map_into_iter_owned() {
    // Test consuming IntoIterator for owned DiscriminantMap
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Consume the split and verify we get the right iterator
    let items: Vec<_> = split.into_iter().collect();

    // Should have entries for a_disc and b_disc
    assert_eq!(items.len(), 2);

    // Check that both discriminants are present and have the right content
    let mut found_a = false;
    let mut found_b = false;

    for (disc, vec) in items {
        if disc == a_disc {
            assert_eq!(vec.len(), 2);
            assert!(vec.iter().all(|e| matches!(e, E::A(_))));
            found_a = true;
        } else if disc == b_disc {
            assert_eq!(vec.len(), 1);
            assert!(vec.iter().all(|e| matches!(e, E::B(_))));
            found_b = true;
        }
    }

    assert!(found_a && found_b, "Both discriminants should be present");
}

#[test]
fn discriminant_map_into_iter_shared_ref() {
    // Test IntoIterator for &DiscriminantMap yields borrowed slices
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Iterate via shared reference — can be done multiple times
    {
        let mut count = 0;
        for (_disc, _items) in &split {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    // Can iterate again
    {
        let items: Vec<_> = (&split).into_iter().collect();
        assert_eq!(items.len(), 2);
        for (_disc, group) in items {
            assert!(!group.is_empty());
        }
    }
}

#[test]
fn discriminant_map_into_iter_mutable_ref() {
    // Test IntoIterator for &mut DiscriminantMap yields mutable slices  
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::A(3)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Modify through mutable iteration
    for (_disc, items) in &mut split {
        for item in items {
            if let E::A(v) = item {
                *v += 10;
            }
        }
    }

    // Verify modifications persisted
    assert_eq!(data[0], E::A(11));
    assert_eq!(data[1], E::A(12));
    assert_eq!(data[2], E::A(13));
}

#[test]
fn discriminant_map_into_iter_with_others() {
    // Test that others don't appear in the IntoIterator output
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::C, E::B("hello".into()), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    // Only entries for requested discriminants are in the iterator
    let entries: Vec<_> = split.into_iter().map(|(k, _v)| k).collect();
    assert_eq!(entries.len(), 1);
    assert!(entries.iter().all(|&d| d == a_disc));
}

// ── SplitWithExtractor IntoIterator tests ─────────────────────────────────────

#[test]
fn split_with_extractor_into_iter_owned() {
    // Test consuming IntoIterator for owned SplitWithExtractor
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Consume and collect
    let items: Vec<_> = ex.into_iter().collect();
    assert_eq!(items.len(), 2);

    // SplitWithExtractor delegates to inner DiscriminantMap
    for (disc, vec) in items {
        if disc == a_disc {
            assert_eq!(vec.len(), 2);
        } else if disc == b_disc {
            assert_eq!(vec.len(), 1);
        }
    }
}

#[test]
fn split_with_extractor_into_iter_shared_ref() {
    // Test IntoIterator for &SplitWithExtractor
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Multiple iterations via shared ref
    {
        let count: usize = (&ex).into_iter().count();
        assert_eq!(count, 1);
    }

    // Can iterate multiple times
    for (_disc, items) in &ex {
        assert_eq!(items.len(), 2);
    }
}

#[test]
fn split_with_extractor_into_iter_mutable_ref() {
    // Test IntoIterator for &mut SplitWithExtractor
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(5), E::A(6)];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Modify through mutable iteration
    for (_disc, items) in &mut ex {
        for item in items {
            if let E::A(v) = item {
                *v *= 2;
            }
        }
    }

    // Verify modifications
    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::A(12));
}

#[test]
fn into_iter_preserves_order_across_multiple_calls() {
    // Verify that shared/mutable ref iteration maintains consistency
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::A(3)];
    let mut split = split_by_discriminant(&mut data[..], &[a_disc]);

    // First pass: iterate and collect values
    let first_pass: Vec<i32> = (&split)
        .into_iter()
        .flat_map(|(_disc, items)| {
            items.iter().filter_map(|e| {
                if let E::A(v) = e { Some(*v) } else { None }
            })
        })
        .collect();

    // Second pass: mutate and collect
    let second_pass: Vec<i32> = (&mut split)
        .into_iter()
        .flat_map(|(_disc, items)| {
            items.into_iter().filter_map(|e| {
                if let E::A(v) = e {
                    *v += 10;
                    Some(*v)
                } else {
                    None
                }
            })
        })
        .collect();

    // Verify values in expected order
    assert_eq!(first_pass, vec![1, 2, 3]);
    assert_eq!(second_pass, vec![11, 12, 13]);
}

#[test]
fn into_iter_empty_groups() {
    // Test iteration when some groups exist but are empty
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut data = [E::C];  // No A or B variants
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);

    // Even though we requested A and B, no groups should be created for absent variants
    let count: usize = split.into_iter().count();
    assert_eq!(count, 0);
}

#[test]
fn into_iter_single_group() {
    // Test iteration with only one discriminant group
    let a_disc = discriminant(&E::A(0));

    let mut data = [E::A(1), E::A(2), E::C];
    let split = split_by_discriminant(&mut data[..], &[a_disc]);

    let items: Vec<_> = split.into_iter().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].0, a_disc);
    assert_eq!(items[0].1.len(), 2);
}
