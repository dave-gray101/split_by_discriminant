// Fixture file for emulate_derive_macro_expansion("ExtractFrom", ...).
// NOT compiled as a test binary — opened as a File by tarpaulin.rs at runtime.
// Each item exercises a distinct code path in derive_extract_from.rs.

// ── Strategy::Simple (1 non-ref single-field variant) ────────────────────────
#[derive(ExtractFrom)]
enum SimpleStratEnum { A(i32) }

// ── Strategy::Variant (multiple variants, unique single-field types) ──────────
#[derive(ExtractFrom)]
enum VariantStratEnum { A(i32), B(String) }

// ── Strategy::Extract: multi-field variant ───────────────────────────────────
#[derive(ExtractFrom)]
enum MultiFieldEnum { A(i32, String), B(f64) }

// ── Unit variant skipping (Fields::Unit => continue) ─────────────────────────
#[derive(ExtractFrom)]
enum WithUnitVariant { Unit, A(i32), B(String) }

// ── Strategy::Extract: duplicate field type forces Extract ───────────────────
#[derive(ExtractFrom)]
enum DupTypeEnum { A(i32), B(i32) }

// ── Strategy::Extract: reference field (is_reference → true) ─────────────────
#[derive(ExtractFrom)]
enum RefFieldEnum { A(&'static i32), B(String) }

// ── Strategy::Extract: named (struct-like) fields (is_named: true paths) ─────
#[derive(ExtractFrom)]
enum NamedFieldsEnum { A { x: i32, y: String }, B(f64) }

// ── skip_empty flag on empty enum ────────────────────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(skip_empty)]
enum SkipEmptyEnum {}

// ── Error: empty enum without skip_empty ─────────────────────────────────────
#[derive(ExtractFrom)]
enum EmptyNoSkipEnum {}

// ── Error: non-enum item (struct) ─────────────────────────────────────────────
#[derive(ExtractFrom)]
struct NotAnEnumStruct { x: i32 }

// ── Custom extractor: plain name — no placeholder (emit_extractor_struct=false)
#[derive(ExtractFrom)]
#[extract_from(extractor = "ExternalExtractor")]
enum PlainExtractorEnum { A(i32) }

// ── Custom extractor: positional {} placeholder ──────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(extractor = "Custom{}Ex")]
enum PositionalExtractorEnum { A(i32) }

// ── Custom extractor: named {enum} placeholder ───────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(extractor = "Custom{enum}Ex")]
enum NamedEnumExtractorEnum { A(i32) }

// ── Error: {variant} placeholder not valid in extractor name ─────────────────
#[derive(ExtractFrom)]
#[extract_from(extractor = "Custom{variant}Ex")]
enum VariantPlaceholderInExtractorName { A(i32) }

// ── Error: unknown placeholder in extractor name ─────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(extractor = "Custom{unknown}Ex")]
enum UnknownPlaceholderInExtractorName { A(i32) }

// ── Error: resulting extractor name is not a valid Rust identifier ────────────
#[derive(ExtractFrom)]
#[extract_from(extractor = "123invalid")]
enum InvalidIdentExtractorName { A(i32) }

// ── Global selector: positional {} {} (1st=enum, 2nd=variant) ────────────────
#[derive(ExtractFrom)]
#[extract_from(selector = "Select{}{}")]
enum GlobalPosSelectorEnum { A(i32, String), B(f64, i32) }

// ── Global selector: named {enum}{variant} ───────────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(selector = "Sel{enum}{variant}")]
enum GlobalNamedSelectorEnum { A(i32, String) }

// ── Per-variant selector override (is_generated: false path) ─────────────────
#[derive(ExtractFrom)]
enum PerVariantSelectorEnum {
    #[extract_from(selector = "ExternalSel")]
    A(i32, String),
}

// ── Error: unknown key on enum-level extract_from attribute ───────────────────
#[derive(ExtractFrom)]
#[extract_from(unknown_key = "val")]
enum UnknownKeyOnEnumAttr { A(i32) }

// ── Error: unknown key on variant-level extract_from attribute ────────────────
#[derive(ExtractFrom)]
enum UnknownKeyOnVariantAttr {
    #[extract_from(bad_attr = "v")]
    A(i32),
}

// ── Error: too many positional selector placeholders (3rd {}) ────────────────
#[derive(ExtractFrom)]
#[extract_from(selector = "Sel{}{}{}")]
enum TooManyPosSelectorPlaceholders { A(i32, String) }

// ── Error: unknown named selector placeholder ────────────────────────────────
#[derive(ExtractFrom)]
#[extract_from(selector = "Sel{unknown}")]
enum UnknownNamedSelectorPlaceholder { A(i32, String) }

// ── Error: selector string produces invalid identifier ───────────────────────
#[derive(ExtractFrom)]
#[extract_from(selector = "123")]
enum InvalidSelectorIdentifier { A(i32, String) }

// ── Error: unclosed { at end of selector string (empty inner → error) ─────────
#[derive(ExtractFrom)]
#[extract_from(selector = "Sel{")]
enum UnclosedSelectorBrace { A(i32, String) }
