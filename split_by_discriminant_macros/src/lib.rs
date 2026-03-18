use proc_macro::TokenStream;

use split_by_discriminant::proc_macro::derive_extract_from;

#[proc_macro_derive(ExtractFrom, attributes(extract_from))]
pub fn extract_from(input: TokenStream) -> TokenStream {
    derive_extract_from(input.into()).into()
}