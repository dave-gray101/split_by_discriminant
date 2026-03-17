// Integration tests for multi-field extraction via tuple Output<'a>.
//
// These tests verify an extraction pattern that was impossible with the old
// `extract_with` API (which required a single `&'a mut U` output) and is the
// primary motivator for the GAT `type Output<'a>` in `ExtractFrom`.

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor, ExtractFrom};
use std::mem::discriminant;

#[derive(Debug, PartialEq)]
enum E {
    Pair(i32, String),
    Single(u8),
    Other,
}

struct PairExtractor;
struct SelectPair;

impl ExtractFrom<E, SelectPair> for PairExtractor {
    type Output<'a> = (&'a mut i32, &'a mut String);
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<Self::Output<'a>> {
        if let E::Pair(n, s) = t { Some((n, s)) } else { None }
    }
}

/// `extract_gat::<SelectPair>` borrows both fields of `E::Pair` simultaneously.
/// The returned lifetime is tied to the `SplitWithExtractor` borrow.
#[test]
fn multi_field_extract_reborrow() {
    let mut data = vec![
        E::Pair(1, "a".into()),
        E::Single(9),
        E::Pair(2, "b".into()),
        E::Other,
    ];
    let pair_disc = discriminant(&E::Pair(0, String::new()));

    let split = split_by_discriminant(&mut data, &[pair_disc]);
    let mut ex = SplitWithExtractor::new(split, PairExtractor);

    {
        let mut pairs: Vec<(&mut i32, &mut String)> = ex.extract_gat::<SelectPair>(pair_disc).unwrap();
        assert_eq!(pairs.len(), 2);
        *pairs[0].0 += 10;
        pairs[0].1.push('!');
        *pairs[1].0 += 20;
        pairs[1].1.push('?');
    }

    assert_eq!(data[0], E::Pair(11, "a!".into()));
    assert_eq!(data[2], E::Pair(22, "b?".into()));
}

/// `take_extracted::<SelectPair>` moves the pairs out with the full `'items`
/// lifetime, allowing them to outlive the `SplitWithExtractor`.
#[test]
fn multi_field_take_extracted_full_lifetime() {
    let mut data = [
        E::Pair(10, "x".into()),
        E::Other,
        E::Pair(20, "y".into()),
    ];
    let pair_disc = discriminant(&E::Pair(0, String::new()));

    let mut pairs: Vec<(&mut i32, &mut String)> = {
        let split = split_by_discriminant(&mut data[..], &[pair_disc]);
        let mut ex = SplitWithExtractor::new(split, PairExtractor);
        ex.take_extracted::<SelectPair>(pair_disc).unwrap()
        // `ex` and `split` are dropped here; `pairs` carries the original `'items` lifetime
    };

    assert_eq!(pairs.len(), 2);
    *pairs[0].0 = 99;
    *pairs[1].0 = 88;
    drop(pairs);

    assert_eq!(data[0], E::Pair(99, "x".into()));
    assert_eq!(data[2], E::Pair(88, "y".into()));
}

/// `get_mut` + `iter_mut` with a closure — the migration pattern from §3 of the
/// plan, exercising the same multi-field output without a typed extractor.
#[test]
fn multi_field_get_mut_iter_form() {
    let mut data = [
        E::Pair(1, "hello".into()),
        E::Single(7),
        E::Pair(2, "world".into()),
    ];
    let pair_disc = discriminant(&E::Pair(0, String::new()));

    let mut split = split_by_discriminant(&mut data[..], &[pair_disc]);
    let mut pairs: Vec<(&mut i32, &mut String)> = if let Some(entries) = split.get_mut(pair_disc) {
        entries
            .iter_mut()
            .filter_map(|e| if let E::Pair(n, s) = e { Some((n, s)) } else { None })
            .collect()
    } else {
        vec![]
    };

    assert_eq!(pairs.len(), 2);
    *pairs[0].0 = 42;
    *pairs[1].1 = "rust".into();
    drop(pairs);

    assert_eq!(data[0], E::Pair(42, "hello".into()));
    assert_eq!(data[2], E::Pair(2, "rust".into()));
}
