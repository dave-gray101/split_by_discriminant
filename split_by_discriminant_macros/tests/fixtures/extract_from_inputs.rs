// Fixture file for emulate_functionlike_macro_expansion("extract_from", ...).
// NOT compiled as a test binary — opened as a File by tarpaulin.rs at runtime.
//
// Each extract_from! call receives the full enum definition as its token stream,
// which fn_extract_from passes to derive_extract_from internally.

// Simple strategy
extract_from! { enum ExSimple { A(u32) } }

// Variant strategy
extract_from! { enum ExVariant { A(u32), B(String) } }

// Extract strategy (multi-field)
extract_from! { enum ExMultiField { A(u32, String), B(f64) } }

// Extract strategy (named fields, is_named: true)
extract_from! { enum ExNamedFields { A { x: u32, y: String } } }

// Skip empty
extract_from! {
    #[extract_from(skip_empty)]
    enum ExSkipEmpty {}
}

// Custom extractor name via placeholder
extract_from! {
    #[extract_from(extractor = "Custom{}ExFn")]
    enum ExCustomExtractor { A(u32) }
}

// Custom global selector format
extract_from! {
    #[extract_from(selector = "FnSel{}{}")]
    enum ExCustomSelector { A(u32, String) }
}
