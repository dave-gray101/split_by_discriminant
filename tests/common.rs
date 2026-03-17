// Common definitions reused across multiple integration test files.
// This module is imported via `mod common;` in each test file.

#![allow(dead_code)]

use std::mem::discriminant;

use split_by_discriminant::{ExtractFrom, SimpleExtractFrom, VariantExtractFrom};

#[derive(Debug, PartialEq, Clone)]
pub enum E {
    A(i32),
    B(String),
    C,
}

/// Selector ZST for extracting the `String` field of `E::B`.
///
/// A named selector is needed here because `ComplexExtractor` already uses
/// the default selector `()` (via the `SimpleExtractFrom` blanket) for `E::A`.
/// Every additional extraction target on the same `(E, T)` pair needs its own
/// ZST to keep the impls distinct.
pub struct SelectB;

/// A minimal extractor: implements only [`SimpleExtractFrom<E>`], extracting
/// the `i32` from `E::A`.
///
/// No named selector is needed anywhere — `extract()` infers the return type
/// directly from `SimpleExtractFrom<E>::Output = i32`.  The blanket impl also
/// provides `ExtractFrom<E, ()>` and `TakeFrom<&mut E, ()>` for free.
pub struct SimpleExtractor;

impl SimpleExtractFrom<E> for SimpleExtractor {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

/// A full-featured extractor demonstrating both extraction traits on one type:
///
/// * [`SimpleExtractFrom<E>`] handles `E::A(i32)`.  The blanket automatically
///   provides `ExtractFrom<E, ()>` and `TakeFrom<&mut E, ()>`, so
///   `extract()` works with **no turbofish** and `take_extracted::<()>` works
///   for the full-lifetime consuming path.
///
/// * [`ExtractFrom<E, SelectB>`] handles `E::B(String)`.  A named selector is
///   required because this is a second extraction target on the same `(E, T)`
///   pair — the blanket has already claimed the default `()` slot for `E::A`.
///   Call sites use `extract_gat::<SelectB>` or `take_extracted::<SelectB>`.
///
/// `E::C` is never extracted; it flows into the others bucket untouched.
pub struct ComplexExtractor;

impl SimpleExtractFrom<E> for ComplexExtractor {
    type Output = i32;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut i32> {
        if let E::A(v) = t { Some(v) } else { None }
    }
}

impl ExtractFrom<E, SelectB> for ComplexExtractor {
    type Output<'a> = &'a mut String;
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<Self::Output<'a>> {
        if let E::B(v) = t { Some(v) } else { None }
    }
}

impl VariantExtractFrom<E, String> for ComplexExtractor {
    fn extract_from<'a>(&self, t: &'a mut E) -> Option<&'a mut String> {
        if let E::B(v) = t { Some(v) } else { None }
    }
}

// helper functions for discriminants
pub fn a_disc() -> std::mem::Discriminant<E> {
    discriminant(&E::A(0))
}

pub fn b_disc() -> std::mem::Discriminant<E> {
    discriminant(&E::B(String::new()))
}
