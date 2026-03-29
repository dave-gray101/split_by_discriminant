#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

use split_by_discriminant::proc_macro::{derive_extract_from, fn_extract_from, fn_make_extractor};

#[proc_macro_derive(ExtractFrom, attributes(extract_from))]
pub fn d_extract_from(input: TokenStream) -> TokenStream {
    derive_extract_from(input.into()).into()
}

#[proc_macro]
pub fn extract_from(input: TokenStream) -> TokenStream {
    fn_extract_from(input.into()).into()
}

#[proc_macro]
pub fn make_extractor(input: TokenStream) -> TokenStream {
    fn_make_extractor(input.into()).into()
}