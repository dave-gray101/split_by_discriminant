//! Runtime coverage instrumentation for proc-macro implementations.
//!
//! Proc macros run at compile time and are invisible to tarpaulin's runtime
//! instrumentation.  This file uses `runtime-macros` to re-run each macro
//! implementation function against fixture source files at test time, so that
//! tarpaulin can record line-level hits in `derive_extract_from.rs` and
//! `make_extractor.rs`.
//!
//! The three fixture files in `tests/fixtures/` are plain `.rs` source files
//! containing representative macro invocations.  They are NOT compiled as test
//! binaries; they are opened as `File`s and parsed by `runtime-macros`.

use runtime_macros::{emulate_derive_macro_expansion, emulate_functionlike_macro_expansion};
use split_by_discriminant::proc_macro::{derive_extract_from, fn_extract_from, fn_make_extractor};
use std::fs;
use std::path::Path;

/// Open a fixture file relative to this crate's root (resolved at compile time
/// via `CARGO_MANIFEST_DIR` so it works regardless of the working directory).
fn fixture(relative: &str) -> fs::File {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(relative);
    fs::File::open(&path)
        .unwrap_or_else(|e| panic!("failed to open fixture '{}': {e}", path.display()))
}

/// Drives line coverage of `derive_extract_from` via `#[derive(ExtractFrom)]`.
///
/// The fixture contains items exercising every strategy (Simple, Variant,
/// Extract), every attribute option (extractor name formats, selector formats,
/// skip_empty), and all error-returning paths (non-enum input, unknown keys,
/// invalid placeholders, invalid identifiers).
#[test]
fn tarpaulin_derive_extract_from() {
    emulate_derive_macro_expansion(
        fixture("tests/fixtures/derive_inputs.rs"),
        &[("ExtractFrom", derive_extract_from)],
    )
    .expect("derive_extract_from emulation failed");
}

/// Drives line coverage of `fn_extract_from` via `extract_from! { ... }`.
///
/// `fn_extract_from` re-emits the enum definition alongside the generated
/// impls.  The fixture covers Simple / Variant / Extract strategies plus the
/// custom-extractor and custom-selector attribute paths.
#[test]
fn tarpaulin_fn_extract_from() {
    emulate_functionlike_macro_expansion(
        fixture("tests/fixtures/extract_from_inputs.rs"),
        &[("extract_from", fn_extract_from)],
    )
    .expect("fn_extract_from emulation failed");
}

/// Drives line coverage of `fn_make_extractor` via `make_extractor! { ... }`.
///
/// The fixture covers auto-naming, explicit names, both key aliases
/// (`function` / `fn_name`, `ref_function` / `ref_fn_name`), the
/// `trait_name` bound, the `ref_fn_name` companion function branch, combined
/// `trait_name + ref_fn_name`, and the unknown-key error path.
#[test]
fn tarpaulin_make_extractor() {
    emulate_functionlike_macro_expansion(
        fixture("tests/fixtures/make_extractor_inputs.rs"),
        &[("make_extractor", fn_make_extractor)],
    )
    .expect("fn_make_extractor emulation failed");
}
