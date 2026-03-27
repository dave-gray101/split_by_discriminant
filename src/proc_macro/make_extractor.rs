use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use heck::{ToSnakeCase, ToUpperCamelCase};

struct MakeExtractorArgs {
    enum_name: Ident,
    extractor_name: Option<Ident>,
    function_name: Option<Ident>,
    trait_name: Option<Ident>,
}

impl syn::parse::Parse for MakeExtractorArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let enum_name: Ident = input.parse()?;
        let mut extractor_name = None;
        let mut function_name = None;
        let mut trait_name = None;

        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value: Ident = input.parse()?;

            match key.to_string().as_str() {
                "extractor" => extractor_name = Some(value),
                "fn_name" | "fn" | "function" => function_name = Some(value),
                "trait" | "trait_name" => trait_name = Some(value),
                _ => {
                    let msg = format!("unexpected key: `{key}`. Expected `extractor`, `fn_name`/`fn`/`function`, or `trait`/`trait_name`.");
                    return Err(syn::Error::new_spanned(
                        key,
                        msg),
                    );
                }
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(MakeExtractorArgs {
            enum_name,
            extractor_name,
            function_name,
            trait_name,
        })
    }
}


pub fn fn_make_extractor(input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<MakeExtractorArgs>(input) {
        Ok(v) => v,
        Err(err) => return err.to_compile_error(),
    };

    let enum_name = &args.enum_name;
    let enum_name_string = enum_name.to_string();

    let extractor_name = args.extractor_name.unwrap_or_else(|| {
        format_ident!("{}Extractor", enum_name_string.to_upper_camel_case())
    });

    let function_name = args.function_name.unwrap_or_else(|| {
        format_ident!("make_{}_extractor", enum_name_string.to_snake_case())
    });

    let t_trait_bound = if let Some(trait_name) = &args.trait_name {
        quote! { T: #trait_name, }
    } else {
        quote! {}
    };

    let doc1 = format!(
        "Create a [`SplitWithExtractor`] for `{enum}` values using extractor [`{extractor}`].",
        extractor = extractor_name,
        enum = enum_name,
    );

    let trait_docs = if let Some(trait_name) = &args.trait_name {
        format!("Requires `T: {}` for the generated function signature.", trait_name)
    } else {
        "No additional trait bound is required.".to_string()
    };

    let doc2 = format!(
        "This is a small helper around [`split_by_discriminant()`] + [`SplitWithExtractor::new`]. {trait_docs}",
        trait_docs = trait_docs,
    );

    let doc3 = "See https://docs.rs/split_by_discriminant/latest/split_by_discriminant/ for full API docs.";

    quote! {
        #[doc = #doc1]
        #[doc = #doc2]
        #[doc = #doc3]
        pub fn #function_name<T, I, R>(items: I, discs: &[std::mem::Discriminant<T>]) -> split_by_discriminant::SplitWithExtractor<T, R, R, #extractor_name>
        where
            #t_trait_bound
            I: IntoIterator<Item = R>,
            R: std::borrow::BorrowMut<T>,
        {
            let split = split_by_discriminant::split_by_discriminant(items, discs);
            split_by_discriminant::SplitWithExtractor::new(split, #extractor_name)
        }
    }
    .into()
}
