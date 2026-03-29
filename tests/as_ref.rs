mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;

// ── map_as_ref Tests ──────────────────────────────────────────────────────

#[test]
fn map_as_ref_closure_immutable_extraction() {
    let data = [E::A(1), E::B("hello".into()), E::A(2), E::C];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let ints: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    assert_eq!(*ints[0], 1);
    assert_eq!(*ints[1], 2);

    let strs: Vec<&String> = ex
        .map_as_ref(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strs.len(), 1);
    assert_eq!(strs[0], "hello");
}

#[test]
fn map_as_ref_no_turbofish() {
    let data = [E::A(1), E::A(2), E::B(String::new())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Fully inferred from binding
    let result: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn map_as_ref_with_filter() {
    let data = [E::A(1), E::A(2), E::A(3), E::B(String::new())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Filter to odd numbers using closure return
    let odds: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| {
            if let E::A(v) = e {
                if v % 2 == 1 {
                    Some(v)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(odds.len(), 2);
    assert_eq!(*odds[0], 1);
    assert_eq!(*odds[1], 3);
}

#[test]
fn map_as_ref_returns_none_for_absent_discriminant() {
    let data = [E::A(1), E::B(String::new())];
    let a_disc = a_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let result = ex.map_as_ref(c_disc, |e| if let E::A(v) = e { Some(v) } else { None });
    assert!(result.is_none());
}

#[test]
fn map_as_ref_empty_group() {
    let data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // b_disc was never populated, so it's empty but still exists
    let result = ex.map_as_ref(b_disc, |e| if let E::B(_) = e { Some(e) } else { None });
    // Since b_disc wasn't in the original data, we get None
    assert!(result.is_none());
}

#[test]
fn map_as_ref_multiple_calls_same_split() {
    let data = [E::A(1), E::B("test".into()), E::A(2), E::B("again".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Multiple separate immutable accesses are allowed
    let ints1: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    let strs: Vec<&String> = ex
        .map_as_ref(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    let ints2: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();

    assert_eq!(ints1.len(), 2);
    assert_eq!(strs.len(), 2);
    assert_eq!(ints2.len(), 2);
}

#[test]
fn map_as_ref_doesnt_require_mut_borrow() {
    let data = [E::A(1), E::A(2), E::B(String::new())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // map_as_ref takes &self, not &mut self — multiple calls allowed without mut
    let result1: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    let result2: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();

    assert_eq!(result1.len(), 2);
    assert_eq!(result2.len(), 2);
}

#[test]
fn map_as_ref_with_complex_extraction_logic() {
    #[derive(Debug, Clone)]
    enum Value {
        Single(i32),
        Pair(i32, i32),
    }

    let data = [
        Value::Single(1),
        Value::Pair(2, 3),
        Value::Single(4),
        Value::Pair(5, 6),
    ];
    let single_disc = discriminant(&Value::Single(0));
    let pair_disc = discriminant(&Value::Pair(0, 0));

    let split = split_by_discriminant(&data[..], &[single_disc, pair_disc]);
    let ex = SplitWithExtractor::new(split, ());

    // Extract first component of each value
    let firsts: Vec<&i32> = ex
        .map_as_ref(single_disc, |v| {
            if let Value::Single(x) = v {
                Some(x)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(firsts.len(), 2);
    assert_eq!(*firsts[0], 1);
    assert_eq!(*firsts[1], 4);

    // Extract first component of pairs
    let pair_firsts: Vec<&i32> = ex
        .map_as_ref(pair_disc, |v| {
            if let Value::Pair(x, _) = v {
                Some(x)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(pair_firsts.len(), 2);
    assert_eq!(*pair_firsts[0], 2);
    assert_eq!(*pair_firsts[1], 5);

    // Extract second component of pairs (reads field `1` of Pair)
    let pair_seconds: Vec<&i32> = ex
        .map_as_ref(pair_disc, |v| {
            if let Value::Pair(_, y) = v {
                Some(y)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(*pair_seconds[0], 3);
    assert_eq!(*pair_seconds[1], 6);
}

#[test]
fn map_as_ref_all_none_filtering() {
    let data = [E::A(1), E::A(2), E::A(3)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Filter that returns None for everything
    let empty: Vec<&i32> = ex
        .map_as_ref(a_disc, |_e| None)
        .unwrap();
    assert_eq!(empty.len(), 0);
}

#[test]
fn map_as_ref_all_some_passthrough() {
    let data = [E::A(1), E::A(2), E::A(3)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // Passthrough closure that always succeeds
    let all: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(*all[0], 1);
    assert_eq!(*all[1], 2);
    assert_eq!(*all[2], 3);
}

#[test]
fn map_as_ref_lifetime_stays_tied_to_call() {
    let data = [E::A(1), E::A(2), E::B(String::new())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // References are tied to the call site lifetime
    {
        let refs: Vec<&i32> = ex
            .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
            .unwrap();
        assert_eq!(refs.len(), 2);
        // refs are valid here
        assert_eq!(*refs[0], 1);
    } // refs go out of scope here

    // Can call again and get new references
    let refs2: Vec<&i32> = ex
        .map_as_ref(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(refs2.len(), 2);
}

// ── map_as_ref_multiple Tests ─────────────────────────────────────────────

#[test]
fn map_as_ref_multiple_basic() {
    let data = [E::A(1), E::B("hello".into()), E::A(2), E::B("world".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let result = ex.map_as_ref_multiple(
        &[a_disc, b_disc],
        |e| if let E::A(v) = e { Some(v) } else { None },
    );

    assert_eq!(result.len(), 2);
    let ints = result.get(&a_disc).unwrap();
    assert_eq!(ints.len(), 2);
    assert_eq!(*ints[0], 1);
    assert_eq!(*ints[1], 2);

    // b_disc is in results but E::A extractor returns None for it → empty Vec
    let strs = result.get(&b_disc).unwrap();
    assert_eq!(strs.len(), 0);
}

#[test]
fn map_as_ref_multiple_partial_match() {
    // Only a_disc is present in the data; c_disc is not in the split at all
    let data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let result = ex.map_as_ref_multiple(
        &[a_disc, c_disc],
        |e| if let E::A(v) = e { Some(v) } else { None },
    );

    // Only a_disc was in the split; c_disc is absent from the map
    assert_eq!(result.len(), 1);
    assert!(result.contains_key(&a_disc));
    assert!(!result.contains_key(&c_disc));

    let ints = result.get(&a_disc).unwrap();
    assert_eq!(ints.len(), 2);
}

#[test]
fn map_as_ref_multiple_empty_ids() {
    let data = [E::A(1), E::A(2)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let result = ex.map_as_ref_multiple(
        &[],
        |e| if let E::A(v) = e { Some(v) } else { None },
    );

    assert_eq!(result.len(), 0);
}

#[test]
fn map_as_ref_multiple_does_not_require_mut_borrow() {
    let data = [E::A(10), E::B("x".into()), E::A(20)];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // &self — can call twice without mut
    let r1 = ex.map_as_ref_multiple(
        &[a_disc],
        |e| if let E::A(v) = e { Some(v) } else { None },
    );
    let r2 = ex.map_as_ref_multiple(
        &[a_disc],
        |e| if let E::A(v) = e { Some(v) } else { None },
    );

    assert_eq!(r1[&a_disc].len(), 2);
    assert_eq!(r2[&a_disc].len(), 2);
}

// ── as_ref_simple / as_ref / as_ref_with Tests ───────────────────────────
//
// These test the trait-based immutable read methods.  Unlike the closure-based
// map_as_ref, these use SimpleReadFrom / VariantReadFrom / ReadFrom impls
// defined on the extractor in tests/common.rs.

#[test]
fn as_ref_simple_basic() {
    let mut data = [E::A(1), E::A(2), E::B(String::new())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let ints: Vec<&i32> = ex.as_ref_simple(a_disc).unwrap();
    assert_eq!(ints.len(), 2);
    assert_eq!(*ints[0], 1);
    assert_eq!(*ints[1], 2);
}

#[test]
fn as_ref_simple_does_not_require_mut_borrow() {
    // &self — can call multiple times without mut
    let mut data = [E::A(10), E::A(20)];
    let a_disc = a_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let r1: Vec<&i32> = ex.as_ref_simple(a_disc).unwrap();
    let r2: Vec<&i32> = ex.as_ref_simple(a_disc).unwrap();
    assert_eq!(r1.len(), 2);
    assert_eq!(r2.len(), 2);
    assert_eq!(*r1[0], 10);
}

#[test]
fn as_ref_simple_absent_discriminant_returns_none() {
    let mut data = [E::A(1)];
    let a_disc = a_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    assert!(ex.as_ref_simple(c_disc).is_none());
}

/// Key advantage: works with immutable source data (G = &E, no BorrowMut needed).
/// as_mut_simple would not compile here — only as_ref_simple does.
#[test]
fn as_ref_simple_works_with_immutable_source_data() {
    let data = [E::A(1), E::A(2), E::B(String::new())];
    let a_disc = a_disc();
    // &data[..] → iterator yields &E → G = &E (no BorrowMut<E>)
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let ints: Vec<&i32> = ex.as_ref_simple(a_disc).unwrap();
    assert_eq!(ints.len(), 2);
    // Data is still owned by `data`, no mutation occurred
    assert_eq!(data[0], E::A(1));
}

#[test]
fn as_ref_u_inferred_from_binding() {
    // ComplexExtractor: SimpleReadFrom<E> → blanket gives VariantReadFrom<E, i32>
    //                   VariantReadFrom<E, String> explicitly implemented
    let mut data = [E::A(1), E::B("hello".into()), E::A(2)];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, ComplexExtractor);

    // U = i32 from binding (via blanket from SimpleReadFrom)
    {
        let ints: Vec<&i32> = ex.as_ref(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        assert_eq!(*ints[0], 1);
    }

    // U = String from binding (via explicit VariantReadFrom impl)
    {
        let strs: Vec<&String> = ex.as_ref(b_disc).unwrap();
        assert_eq!(strs.len(), 1);
        assert_eq!(strs[0], "hello");
    }
}

#[test]
fn as_ref_with_selector() {
    let mut data = [E::B("world".into()), E::A(5), E::B("rust".into())];
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[b_disc]);
    let ex = SplitWithExtractor::new(split, ComplexExtractor);

    // ReadFrom<E, SelectB> impl in common.rs
    let strs: Vec<&String> = ex.as_ref_with::<SelectB>(b_disc).unwrap();
    assert_eq!(strs.len(), 2);
    assert_eq!(strs[0], "world");
    assert_eq!(strs[1], "rust");
}

// ── as_ref_multiple_* Tests ───────────────────────────────────────────────

#[test]
fn as_ref_multiple_simple_basic() {
    let mut data = [E::A(1), E::A(2), E::B("x".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let result = ex.as_ref_multiple_simple(&[a_disc, b_disc]);
    assert_eq!(result.len(), 2);

    // a_disc present, SimpleReadFrom returns Some for E::A
    let ints = result.get(&a_disc).unwrap();
    assert_eq!(ints.len(), 2);
    assert_eq!(*ints[0], 1);
    assert_eq!(*ints[1], 2);

    // b_disc present but SimpleExtractor returns None for E::B → empty Vec
    let empty = result.get(&b_disc).unwrap();
    assert_eq!(empty.len(), 0);
}

#[test]
fn as_ref_multiple_u_basic() {
    let mut data = [E::A(1), E::B("a".into()), E::A(2), E::B("b".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, ComplexExtractor);

    // U = String inferred from binding
    let result: std::collections::HashMap<_, Vec<&String>> = {
        // Use as_ref_multiple via VariantReadFrom<E, String>
        let r = ex.as_ref_multiple(&[b_disc]);
        r.into_iter().collect()
    };
    let strs = result.get(&b_disc).unwrap();
    assert_eq!(strs.len(), 2);
    assert_eq!(strs[0].as_str(), "a");
}

#[test]
fn as_ref_multiple_with_selector_basic() {
    let mut data = [E::B("x".into()), E::A(9), E::B("y".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, ComplexExtractor);

    let result = ex.as_ref_multiple_with::<SelectB>(&[a_disc, b_disc]);
    assert_eq!(result.len(), 2);

    // b_disc: ReadFrom<E, SelectB> gives &String
    let strs = result.get(&b_disc).unwrap();
    assert_eq!(strs.len(), 2);
    assert_eq!(*strs[0], "x");

    // a_disc: ReadFrom<E, SelectB> returns None for E::A → empty Vec
    let empty = result.get(&a_disc).unwrap();
    assert_eq!(empty.len(), 0);
}

#[test]
fn as_ref_multiple_simple_partial_match() {
    let mut data = [E::A(7)];
    let a_disc = a_disc();
    let c_disc = c_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    // c_disc was never in the split — should not appear in results
    let result = ex.as_ref_multiple_simple(&[a_disc, c_disc]);
    assert_eq!(result.len(), 1);
    assert!(result.contains_key(&a_disc));
    assert!(!result.contains_key(&c_disc));
}

#[test]
fn as_ref_methods_do_not_invalidate_each_other() {
    // Multiple &self calls with different selectors on the same extractor are valid
    let mut data = [E::A(3), E::B("z".into())];
    let a_disc = a_disc();
    let b_disc = b_disc();
    let split = split_by_discriminant(&mut data[..], &[a_disc, b_disc]);
    let ex = SplitWithExtractor::new(split, ComplexExtractor);

    let ints: Vec<&i32>    = ex.as_ref_simple(a_disc).unwrap();
    let strs: Vec<&String> = ex.as_ref_with::<SelectB>(b_disc).unwrap();
    // Both references are alive simultaneously — no borrow conflict
    assert_eq!(*ints[0], 3);
    assert_eq!(strs[0].as_str(), "z");
}

/// Exercises the `SimpleReadFrom → ReadFrom<T, ()>` blanket impl via `as_ref_with::<()>`.
///
/// `SimpleExtractor` implements `SimpleReadFrom<E>`, which blankets
/// `ReadFrom<E, ()>`.  Calling `as_ref_with::<()>` routes through that blanket,
/// covering the code path that `as_ref_simple` and `as_ref` bypass.
#[test]
fn as_ref_with_unit_selector_uses_simple_read_from_blanket() {
    let data = [E::A(7), E::A(8), E::B("x".into())];
    let a_disc = a_disc();
    let split = split_by_discriminant(&data[..], &[a_disc]);
    let ex = SplitWithExtractor::new(split, SimpleExtractor);

    let ints: Vec<&i32> = ex.as_ref_with::<()>(a_disc).unwrap();
    assert_eq!(ints, vec![&7, &8]);
}

