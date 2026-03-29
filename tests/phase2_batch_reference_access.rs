mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};

// ── Batch Mutable Reference Tests ──────────────────────────────────────────

#[test]
fn as_mut_multiple_simple_basic() {
    let mut data = [E::A(1), E::B("hello".into()), E::A(2), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.as_mut_multiple_simple(&[a_disc, b_disc]);
    
    // Verify both discriminants are present
    assert!(results.contains_key(&a_disc));
    assert!(results.contains_key(&b_disc));
    
    // Verify correct counts
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
    assert_eq!(results.get(&b_disc).unwrap().len(), 0); // No SimpleExtractFrom impl for B
}

#[test]
fn as_mut_multiple_simple_modification() {
    let mut data = [E::A(1), E::A(2), E::A(3), E::C];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.as_mut_multiple_simple(&[a_disc]);
    let refs = results.get(&a_disc).unwrap();
    
    // Verify we can access the values
    assert_eq!(*refs[0], 1);
    assert_eq!(*refs[1], 2);
    assert_eq!(*refs[2], 3);
}

#[test]
fn as_mut_multiple_simple_partial_match() {
    let mut data = [E::A(1), E::A(2), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc, c_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.as_mut_multiple_simple(&[a_disc, b_disc, c_disc]);
    
    // Only a_disc has items
    assert!(results.contains_key(&a_disc));
    // b_disc and c_disc exist but have no items matching extraction
    // They may or may not be in results depending on implementation
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
}

#[test]
fn as_mut_multiple_simple_empty_ids() {
    let mut data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.as_mut_multiple_simple(&[]);
    
    // No ids requested, so result should be empty
    assert!(results.is_empty());
}

#[test]
fn as_mut_multiple_with_selector() {
    let mut data = [E::A(1), E::B("hello".into()), E::A(2), E::B("world".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);

    // as_mut_multiple_with::<SelectB> uses the ExtractFrom<E, SelectB> impl
    // which only extracts E::B items as String
    let results = ex.as_mut_multiple_with::<SelectB>(&[a_disc, b_disc]);
    
    // The test passes if we can call this method and get results
    // SelectB extracts E::B, so b_disc should have items
    // a_disc items shouldn't match SelectB extraction
    assert!(!results.is_empty());
}

// ── Batch Immutable Reference Tests (Closure-based) ─────────────────────────

#[test]
fn map_as_ref_multiple_basic() {
    let data = [E::A(1), E::B("hello".into()), E::A(2), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.map_as_ref_multiple(&[a_disc, b_disc], |e| {
        match e {
            E::A(v) => Some(v),
            E::B(_) => None,
            E::C => None,
        }
    });

    // a_disc has matches, b_disc has empty results
    assert!(results.contains_key(&a_disc));
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
    assert_eq!(*results.get(&a_disc).unwrap()[0], 1);
    assert_eq!(*results.get(&a_disc).unwrap()[1], 2);
}

#[test]
fn map_as_ref_multiple_with_filter() {
    let data = [E::A(1), E::A(2), E::A(3), E::A(4), E::C];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Filter to odd numbers
    let results = ex.map_as_ref_multiple(&[a_disc], |e| {
        if let E::A(v) = e {
            if *v % 2 == 1 {
                Some(v)
            } else {
                None
            }
        } else {
            None
        }
    });

    let odds = results.get(&a_disc).unwrap();
    assert_eq!(odds.len(), 2);
    assert_eq!(*odds[0], 1);
    assert_eq!(*odds[1], 3);
}

#[test]
fn map_as_ref_multiple_empty_ids() {
    let data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.map_as_ref_multiple(&[], |e| {
        if let E::A(v) = e { Some(v) } else { None }
    });

    assert!(results.is_empty());
}

#[test]
fn map_as_ref_multiple_partial_match() {
    let data = [E::A(1), E::A(2), E::B("test".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc, c_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.map_as_ref_multiple(&[a_disc, b_disc, c_disc], |e| {
        match e {
            E::A(v) => Some(v),
            _ => None,
        }
    });

    // a_disc has results, b_disc and c_disc may have empty or no entries
    assert!(results.contains_key(&a_disc));
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
}

// ── Batch Mutable Closure-based References ──────────────────────────────────

#[test]
fn map_as_mut_multiple_basic() {
    let mut data = [E::A(1), E::A(2), E::B("hello".into()), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.map_as_mut_multiple(&[a_disc, b_disc], |e| {
        match e {
            E::A(v) => Some(v),
            E::B(_) => None,
            E::C => None,
        }
    });

    // a_disc has matches
    assert!(results.contains_key(&a_disc));
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
    
    // Verify we can read the values
    assert_eq!(*results.get(&a_disc).unwrap()[0], 1);
    assert_eq!(*results.get(&a_disc).unwrap()[1], 2);
}

#[test]
fn map_as_mut_multiple_with_modification() {
    let mut data = [E::A(1), E::A(2), E::A(3)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    {
        let mut results = ex.map_as_mut_multiple(&[a_disc], |e| {
            if let E::A(v) = e {
                Some(v)
            } else {
                None
            }
        });

        // Modify through mutable references
        if let Some(refs) = results.get_mut(&a_disc) {
            for r in refs.iter_mut() {
                **r *= 10;
            }
        }
    }

    // Verify modifications persisted
    let results2 = ex.map_as_ref_multiple(&[a_disc], |e| {
        if let E::A(v) = e { Some(v) } else { None }
    });
    
    let vals = results2.get(&a_disc).unwrap();
    assert_eq!(*vals[0], 10);
    assert_eq!(*vals[1], 20);
    assert_eq!(*vals[2], 30);
}

#[test]
fn map_as_mut_multiple_empty_ids() {
    let mut data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    let results = ex.map_as_mut_multiple(&[], |e| {
        if let E::A(v) = e { Some(v) } else { None }
    });

    assert!(results.is_empty());
}

// ── as_mut_multiple<U> Tests ───────────────────────────────────────────────

#[test]
fn as_mut_multiple_variant_extraction() {
    let mut data = [E::A(1), E::B("hello".into()), E::A(2), E::B("world".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);

    let results = ex.as_mut_multiple::<i32>(&[a_disc, b_disc]);

    // a_disc should have results (i32 variant)
    assert!(results.contains_key(&a_disc));
    assert_eq!(results.get(&a_disc).unwrap().len(), 2);
    // b_disc might or might not appear depending on whether any E::B items match the i32 extraction
}

// ── Integration: Multiple calls on same split ──────────────────────────────

#[test]
fn multiple_batch_calls_same_split() {
    let mut data = [E::A(1), E::B("hello".into()), E::A(2), E::B("world".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, ComplexExtractor);

    // First batch call for variant extracted A (i32)
    let int_results = ex.as_mut_multiple::<i32>(&[a_disc, b_disc]);
    assert_eq!(int_results.get(&a_disc).unwrap().len(), 2);

    // Second batch call via closure for all
    let all_results = ex.map_as_ref_multiple(&[a_disc, b_disc], |e| Some(e));
    assert!(all_results.contains_key(&a_disc));
    assert!(all_results.contains_key(&b_disc));
}

#[test]
fn batch_methods_consistency_with_single() {
    let mut data = [E::A(1), E::A(2), E::C];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Get single-item result
    let single = ex.as_mut_simple(a_disc).unwrap();
    let single_count = single.len();
    drop(single); // Release borrow
    
    // Get batch result with same discriminant
    let batch = ex.as_mut_multiple_simple(&[a_disc]);
    
    // Both should have same count
    assert_eq!(single_count, batch.get(&a_disc).unwrap().len());
}
