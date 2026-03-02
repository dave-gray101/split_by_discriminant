// Common definitions reused across multiple integration test files.
// This module is imported via `mod common;` in each test file.

#![allow(dead_code)]

use std::mem::discriminant;

use split_by_discriminant::ExtractFrom;

#[derive(Debug, PartialEq, Clone)]
pub enum E {
    A(i32),
    B(String),
    C,
}

// local extractor type used in many tests
pub struct EExtractor;

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

// helper functions for discriminants
pub fn a_disc() -> std::mem::Discriminant<E> {
    discriminant(&E::A(0))
}

pub fn b_disc() -> std::mem::Discriminant<E> {
    discriminant(&E::B(String::new()))
}
