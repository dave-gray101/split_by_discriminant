// Integration-like unit tests for the library. These are kept in a separate
// file to keep `lib.rs` focused on public API.

use super::*;
use std::mem::discriminant;

#[derive(Debug, PartialEq, Clone)]
enum E {
    A(i32),
    B(String),
    C,
}

// implement `Extract` for every pair of `E` and an inner type we care about
impl Extract<i32> for E {
    fn extract(&mut self) -> Option<&mut i32> {
        if let E::A(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Extract<String> for E {
    fn extract(&mut self) -> Option<&mut String> {
        if let E::B(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[test]
fn split_and_extract() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    // create the split in its own scope so we can drop it before looking
    // at `data` again; the borrow-checker is happy once `split` is gone.
    {
        let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

        // raw access still available
        assert!(split.group(a_disc).unwrap().len() == 2);

        // ergonomic extraction of the inner type
        let mut ints: Vec<&mut i32> = split.extract(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] = 10;

        let mut strings: Vec<&mut String> = split.extract(b_disc).unwrap();
        assert_eq!(strings.len(), 1);
        strings[0].push_str("!");
    }

    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::B("hi!".into()));
}

#[test]
fn vec_and_slice_are_supported() {
    let mut vec = vec![E::A(7), E::B("z".into()), E::A(8)];
    let a_disc = discriminant(&E::A(0));

    // Vec via &mut Vec<T>
    let mut splitv = split_by_discriminant(&mut vec, &[a_disc]);
    assert_eq!(splitv.group(a_disc).unwrap().len(), 2);

    // mutable slice as well
    let mut array = [E::A(3), E::C];
    let mut splits = split_by_discriminant(&mut array[..], &[a_disc]);
    assert_eq!(splits.group(a_disc).unwrap().len(), 1);

    // immutable slice works with the same generic function
    let array2 = [E::A(10), E::B("foo".into())];
    let mut splitr: SplitByDiscriminant<_, &E> =
        split_by_discriminant(&array2[..], &[a_disc]);
    assert_eq!(splitr.group(a_disc).unwrap().len(), 1);
}

#[test]
fn generic_function_infers_reference_kind() {
    let mut data = [E::A(4), E::B("x".into())];
    let a_disc = discriminant(&E::A(0));

    // mutable iterator yields &mut E, so R = &mut E
    let mut s1 = split_by_discriminant(&mut data[..], &[a_disc]);
    // we can call extract because R: BorrowMut
    let _ints: Vec<&mut i32> = s1.extract(a_disc).unwrap();

    let data2 = [E::A(5), E::C];
    // immutable iterator yields &E, hence R = &E
    let mut s2: SplitByDiscriminant<_, &E> =
        split_by_discriminant(&data2[..], &[a_disc]);
    // s2.extract::<i32>(a_disc); // would not compile
    assert_eq!(s2.group(a_disc).unwrap().len(), 1);
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
fn into_parts_returns_components() {
    let mut data = [E::A(1), E::B("x".into()), E::C];
    let a_disc = discriminant(&E::A(0));

    // Splitting only on `A` should leave the other two items in `others`.
    let split = split_by_discriminant(&mut data, &[a_disc]);
    let (groups, others) = split.into_parts();

    // one group for `A`
    assert_eq!(groups.len(), 1);
    assert!(groups.contains_key(&a_disc));
    let group = groups.get(&a_disc).unwrap();
    assert_eq!(group.len(), 1);
    assert_eq!(**group.get(0).unwrap(), E::A(1));

    // the remaining items should be `B` and `C` in order
    assert_eq!(others.len(), 2);
    assert_eq!(*others[0], E::B("x".into()));
    assert_eq!(*others[1], E::C);
}

#[test]
fn extract_nonexistent_discriminant_returns_none() {
    let mut data = [E::A(5), E::B("foo".into())];
    let a_disc = discriminant(&E::A(0));
    let c_disc = discriminant(&E::C);

    // only split on A; C is absent so extract should return None
    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    assert!(split.extract::<i32>(c_disc).is_none());
}

#[test]
fn map_by_discriminant_applies_closures() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    // convert matched elements to their debug string, others to a literal
    let mut split: SplitByDiscriminant<_, String, String> =
        map_by_discriminant(&mut data[..], &[a_disc, b_disc],
            |r: &mut E| format!("MATCH:{:?}", r),
            |r: &mut E| format!("OTHER:{:?}", r),
        );

    assert_eq!(split.group(a_disc).unwrap(), &vec![
        String::from("MATCH:A(1)"),
        String::from("MATCH:A(2)"),
    ]);
    assert_eq!(split.group(b_disc).unwrap(), &vec![String::from("MATCH:B(\"hi\")")]);

    let others = split.into_parts().1;
    assert_eq!(others, vec![String::from("OTHER:C")]);
}

#[test]
fn map_groups_and_map_others_helpers() {
    let mut data = [E::A(5), E::B("x".into()), E::A(6)];
    let a_disc = discriminant(&E::A(0));
    let mut data_clone = data.clone();

    let split1 = split_by_discriminant(&mut data, &[a_disc]);
    let split2 = split_by_discriminant(&mut data_clone, &[a_disc]);

    let counts: Map<_, usize> = split1.map_groups(|v| v.len());
    assert_eq!(counts.get(&a_disc).cloned(), Some(2));

    let other_debug: Vec<String> = split2.map_others(|v| {
        v.into_iter().map(|r| format!("{:?}", r)).collect()
    });
    assert_eq!(other_debug, vec![String::from("B(\"x\")")]);
}
