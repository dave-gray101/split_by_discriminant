use super::*;
use std::mem::discriminant;

#[derive(Debug, PartialEq, Clone)]
enum E {
    A(i32),
    B(String),
    C,
}

// ── Extractor for the local E enum (simulates user_helper for a local type) ──
// EExtractor is local to this crate, so ExtractFrom impls are always legal.
struct EExtractor;

impl ExtractFrom<E, i32> for EExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

impl ExtractFrom<E, String> for EExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut String> {
        if let E::B(v) = t { Some(v) } else { None }
    }
}

#[test]
fn split_with_extractor_and_extract() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    {
        let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
        let mut extractor = SplitWithExtractor::new(split, EExtractor);

        // raw group access still available on SplitWithExtractor directly
        assert_eq!(extractor.group(a_disc).unwrap().len(), 2);

        // ergonomic extraction via the extractor extractor
        let mut ints: Vec<&mut i32> = extractor.extract(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] = 10;

        let mut strings: Vec<&mut String> = extractor.extract(b_disc).unwrap();
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

    // mutable iterator yields &mut E, so R = &mut E; extract_with is available
    let mut s1 = split_by_discriminant(&mut data[..], &[a_disc]);
    let _ints: Vec<&mut i32> = s1
        .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();

    let data2 = [E::A(5), E::C];
    // immutable iterator yields &E, hence R = &E; extract_with not available
    let mut s2: SplitByDiscriminant<_, &E> =
        split_by_discriminant(&data2[..], &[a_disc]);
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

    // only split on A; C is absent — extract_with and SplitWithExtractor::extract
    // must both return None
    let mut split = split_by_discriminant(&mut data, &[a_disc]);
    assert!(split
        .extract_with::<i32, _>(c_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .is_none());

    let split2 = split_by_discriminant(&mut data, &[a_disc]);
    let mut extractor = SplitWithExtractor::new(split2, EExtractor);
    assert!(extractor.extract::<i32>(c_disc).is_none());
}

/// Simulate a downstream crate that owns neither the trait nor the enum type:
/// extract_with takes a plain closure so no trait impl is required at all.
#[test]
fn extract_with_no_trait_impl_required() {
    let mut data = [E::A(1), E::A(2), E::B("hi".into()), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let mut split = split_by_discriminant(&mut data, &[a_disc, b_disc]);

    // closure plays the role of what user_helper would export as a free fn
    let mut ints: Vec<&mut i32> = split
        .extract_with(a_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .unwrap();
    assert_eq!(ints.len(), 2);
    *ints[0] = 99;

    let strings: Vec<&mut String> = split
        .extract_with(b_disc, |e| if let E::B(s) = e { Some(s) } else { None })
        .unwrap();
    assert_eq!(strings.len(), 1);

    // nonexistent discriminant still returns None
    let c_disc = discriminant(&E::C);
    assert!(split
        .extract_with::<i32, _>(c_disc, |e| if let E::A(v) = e { Some(v) } else { None })
        .is_none());

    drop(split);
    assert_eq!(data[0], E::A(99));
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

/// Simulates the four-crate workflow using a genuine std-library enum in place
/// of `external_enums`:
///
///  - `external_enums`       → `std::net::IpAddr`    (foreign; cannot be changed)
///  - `split_by_discriminant` → this crate            (provides the split + SplitWithExtractor)
///  - `user_helper`          → `IpAddrExtractor`      (local glue; owns ExtractFrom impls)
///  - `user_downstream`      → the test functions     (calls SplitWithExtractor::extract)
///
/// `std::net::IpAddr`, `Ipv4Addr`, and `Ipv6Addr` are all defined outside
/// this crate.  The orphan rule would block `impl Extract<Ipv4Addr> for
/// IpAddr` here, but `impl ExtractFrom<IpAddr, Ipv4Addr> for IpAddrExtractor`
/// is always legal because `IpAddrExtractor` is local.
mod foreign_enum_workflow {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::mem::discriminant;

    // ── simulates user_helper ───────────────────────────────────────────────
    pub struct IpAddrExtractor;

    impl ExtractFrom<IpAddr, Ipv4Addr> for IpAddrExtractor {
        fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv4Addr> {
            if let IpAddr::V4(v) = t { Some(v) } else { None }
        }
    }

    impl ExtractFrom<IpAddr, Ipv6Addr> for IpAddrExtractor {
        fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv6Addr> {
            if let IpAddr::V6(v) = t { Some(v) } else { None }
        }
    }

    // ── simulates user_downstream ───────────────────────────────────────────

    #[test]
    fn split_with_extractor_extracts_v4_and_v6() {
        let mut addrs: Vec<IpAddr> = vec![
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), // ::1
            IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)),
        ];

        let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        let v6_disc = discriminant(&IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)));

        {
            let split = split_by_discriminant(&mut addrs, &[v4_disc, v6_disc]);
            let mut extractor = SplitWithExtractor::new(split, IpAddrExtractor);

            // U inferred from E: ExtractFrom<IpAddr, U> — no closure at call site
            {
                let mut v4s: Vec<&mut Ipv4Addr> = extractor.extract(v4_disc).unwrap();
                assert_eq!(v4s.len(), 2);
                // mutate through the reference — visible in addrs after borrow ends
                *v4s[0] = Ipv4Addr::new(10, 0, 0, 1);
            }
            {
                let v6s: Vec<&mut Ipv6Addr> = extractor.extract(v6_disc).unwrap();
                assert_eq!(v6s.len(), 1);
            }

            // group() and extract_with() are available directly on SplitWithExtractor
            assert_eq!(extractor.group(v4_disc).unwrap().len(), 2);
            let _ = extractor.extract_with(v4_disc, |a: &mut IpAddr| {
                if let IpAddr::V4(v) = a { Some(v) } else { None }
            });

            // consuming methods reached via into_inner()
            let (groups, others) = extractor.into_inner().into_parts();
            assert_eq!(groups.get(&v4_disc).unwrap().len(), 2);
            assert_eq!(others.len(), 0);
        }

        // borrow fully released — mutation is visible in original vec
        assert_eq!(addrs[0], IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn split_with_extractor_nonexistent_discriminant_returns_none() {
        let mut addrs: Vec<IpAddr> = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        let v6_disc = discriminant(&IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)));

        // only v4 in the split — v6_disc is absent
        let split = split_by_discriminant(&mut addrs, &[v4_disc]);
        let mut extractor = SplitWithExtractor::new(split, IpAddrExtractor);

        assert!(extractor.extract::<Ipv6Addr>(v6_disc).is_none());
    }

    #[test]
    fn split_with_extractor_into_inner_reaches_consuming_methods() {
        let mut addrs: Vec<IpAddr> = vec![
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
        ];
        let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        let v6_disc = discriminant(&IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)));

        let split = split_by_discriminant(&mut addrs, &[v4_disc, v6_disc]);
        let extractor = SplitWithExtractor::new(split, IpAddrExtractor);

        // map_groups is only on SplitByDiscriminant — reached via into_inner()
        let counts: Map<_, usize> = extractor.into_inner().map_groups(|v| v.len());
        assert_eq!(counts.get(&v4_disc).copied(), Some(1));
        assert_eq!(counts.get(&v6_disc).copied(), Some(1));
    }
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
