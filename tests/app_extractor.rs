// Integration tests demonstrating a single application-wide extractor ZST
// covering two unrelated enum types — the "AppExtractor" pattern from §4 of
// the plan.
//
// The orphan rule is satisfied because the impl lives on the local `AppExtractor`
// type, not on the foreign `T` types. A single extractor instance can therefore
// be shared across multiple independent `SplitWithExtractor` instantiations
// without any duplication of the extractor value.

use std::mem::discriminant;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use split_by_discriminant::{split_by_discriminant, ExtractFrom, SplitWithExtractor};

// ── application-wide enum ─────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum Foo {
    A(i32),
    B(String),
    C,
}

// ── single extractor ZST ──────────────────────────────────────────────────────

struct AppExtractor;

// Selectors for Foo
struct SelectFooA;
struct SelectFooB;

// Selectors for IpAddr
struct SelectV4;
struct SelectV6;

impl ExtractFrom<Foo, SelectFooA> for AppExtractor {
    type Output<'a> = &'a mut i32;
    fn extract_from<'a>(&self, t: &'a mut Foo) -> Option<Self::Output<'a>> {
        if let Foo::A(v) = t { Some(v) } else { None }
    }
}

impl ExtractFrom<Foo, SelectFooB> for AppExtractor {
    type Output<'a> = &'a mut String;
    fn extract_from<'a>(&self, t: &'a mut Foo) -> Option<Self::Output<'a>> {
        if let Foo::B(s) = t { Some(s) } else { None }
    }
}

impl ExtractFrom<IpAddr, SelectV4> for AppExtractor {
    type Output<'a> = &'a mut Ipv4Addr;
    fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<Self::Output<'a>> {
        if let IpAddr::V4(v) = t { Some(v) } else { None }
    }
}

impl ExtractFrom<IpAddr, SelectV6> for AppExtractor {
    type Output<'a> = &'a mut Ipv6Addr;
    fn extract_from<'a>(&self, t: &'a mut IpAddr) -> Option<Self::Output<'a>> {
        if let IpAddr::V6(v) = t { Some(v) } else { None }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// `AppExtractor` resolves the correct `ExtractFrom<Foo, S>` impl for each
/// selector — both selectors work on the same `SplitWithExtractor`.
#[test]
fn app_extractor_works_on_foo() {
    let mut data = vec![
        Foo::A(1),
        Foo::B("hello".into()),
        Foo::A(2),
        Foo::C,
    ];
    let a_disc = discriminant(&Foo::A(0));
    let b_disc = discriminant(&Foo::B(String::new()));

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, AppExtractor);

    {
        let mut ints: Vec<&mut i32> = ex.as_mut_with::<SelectFooA>(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] += 10;
    }
    {
        let mut strings: Vec<&mut String> = ex.as_mut_with::<SelectFooB>(b_disc).unwrap();
        assert_eq!(strings.len(), 1);
        strings[0].push_str("!");
    }

    assert_eq!(data[0], Foo::A(11));
    assert_eq!(data[1], Foo::B("hello!".into()));
    assert_eq!(data[3], Foo::C); // others bucket unaffected
}

/// The same `AppExtractor` type (different instance, but same ZST) handles
/// `IpAddr` — a completely unrelated `T`.
#[test]
fn app_extractor_works_on_ip_addr() {
    let mut addrs: Vec<IpAddr> = vec![
        IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)),
    ];
    let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let v6_disc = discriminant(&IpAddr::V6(Ipv6Addr::UNSPECIFIED));

    let split = split_by_discriminant(&mut addrs, &[v4_disc, v6_disc]);
    let mut ex = SplitWithExtractor::new(split, AppExtractor);

    {
        let mut v4s: Vec<&mut Ipv4Addr> = ex.as_mut_with::<SelectV4>(v4_disc).unwrap();
        assert_eq!(v4s.len(), 2);
        *v4s[0] = Ipv4Addr::new(10, 0, 0, 1);
    }
    {
        let v6s: Vec<&mut Ipv6Addr> = ex.as_mut_with::<SelectV6>(v6_disc).unwrap();
        assert_eq!(v6s.len(), 1);
    }

    assert_eq!(addrs[0], IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
}

/// Both `Foo` and `IpAddr` splits are exercised in a single test function,
/// demonstrating that the compiler resolves the correct impl for each `T`
/// through the same `AppExtractor` type.
#[test]
fn same_test_uses_app_extractor_for_two_distinct_types() {
    // --- Foo split ---
    let mut foos = vec![Foo::A(7), Foo::B("x".into()), Foo::A(8)];
    let a_disc = discriminant(&Foo::A(0));

    let foo_ints: Vec<i32> = {
        let split = split_by_discriminant(&mut foos, &[a_disc]);
        let mut ex = SplitWithExtractor::new(split, AppExtractor);
        let mut refs: Vec<&mut i32> = ex.as_mut_with::<SelectFooA>(a_disc).unwrap();
        refs.iter_mut().map(|r| **r).collect()
    };
    assert_eq!(foo_ints, [7, 8]);

    // --- IpAddr split — same AppExtractor type, completely different T ---
    let mut addrs: Vec<IpAddr> = vec![
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ];
    let v4_disc = discriminant(&IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    let v4_count = {
        let split = split_by_discriminant(&mut addrs, &[v4_disc]);
        let mut ex = SplitWithExtractor::new(split, AppExtractor);
        ex.as_mut_with::<SelectV4>(v4_disc).unwrap().len()
    };
    assert_eq!(v4_count, 1);
}
