use std::mem::discriminant;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use split_by_discriminant::{split_by_discriminant, VariantExtractFrom, SplitWithExtractor};

// ── simulates user_helper ──────────────────────────────────────
// The factory crate defines one extractor type and implements VariantExtractFrom
// once per variant — downstream callers just write:
//   let v4s: Vec<&mut Ipv4Addr> = extractor.extract(v4_disc).unwrap();
// and the compiler infers which impl to use from the binding type.
pub struct IpAddrExtractor;

impl VariantExtractFrom<IpAddr, Ipv4Addr> for IpAddrExtractor {
    fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv4Addr> {
        if let IpAddr::V4(v) = t { Some(v) } else { None }
    }
}

impl VariantExtractFrom<IpAddr, Ipv6Addr> for IpAddrExtractor {
    fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<&'a mut Ipv6Addr> {
        if let IpAddr::V6(v) = t { Some(v) } else { None }
    }
}

// ── tests ────────────────────────────────────────────────────────────────

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

        // U inferred from binding — no closure, no turbofish, no selector ZST
        {
            let mut v4s: Vec<&mut Ipv4Addr> = extractor.as_mut(v4_disc).unwrap();
            assert_eq!(v4s.len(), 2);
            // mutate through the reference — visible in addrs after borrow ends
            *v4s[0] = Ipv4Addr::new(10, 0, 0, 1);
        }
        {
            let v6s: Vec<&mut Ipv6Addr> = extractor.as_mut(v6_disc).unwrap();
            assert_eq!(v6s.len(), 1);
        }

        // get() and extract() are available directly on SplitWithExtractor
        assert_eq!(extractor.get(v4_disc).unwrap().len(), 2);
        // extract again (reborrow, not consumed) to show it works
        let _: Option<Vec<&mut Ipv4Addr>> = extractor.as_mut(v4_disc);

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

    assert!(extractor.as_mut::<Ipv6Addr>(v6_disc).is_none());
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

    // map_all is only on SplitByDiscriminant — reached via into_inner()
    let counts = extractor.into_inner().map_all(|v: Vec<&mut IpAddr>| v.len());
    assert_eq!(counts.get(&v4_disc).copied(), Some(1));
    assert_eq!(counts.get(&v6_disc).copied(), Some(1));
}
