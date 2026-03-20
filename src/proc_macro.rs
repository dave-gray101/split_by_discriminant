use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, Attribute, Data, DeriveInput, Fields, Ident, LitStr, Type};

struct ExtractFromAttrs {
    extractor: Option<String>,
    selector_format: Option<String>,
}

fn parse_extract_from_attr(attrs: &[Attribute]) -> syn::Result<ExtractFromAttrs> {
    let mut out = ExtractFromAttrs {
        extractor: None,
        selector_format: None,
    };

    for attr in attrs {
        if !attr.path().is_ident("extract_from") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("extractor") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                out.extractor = Some(lit.value());
                Ok(())
            } else if meta.path.is_ident("selector") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                out.selector_format = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("unknown attribute key, expected `extractor` or `selector`"))
            }
        })?;
    }

    Ok(out)
}

fn format_extractor_name(format: &str, enum_name: &Ident) -> syn::Result<(String, bool)> {
    // Support placeholder-based formatting in two styles:
    // 1) "Custom{}Extractor" (positional): `{}` is enum
    // 2) "Custom{enum}Extractor" (named)
    // Anything else is emitted verbatim.
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut has_placeholder = false;

    while let Some(ch) = chars.next() {
        if ch == '{' {
            has_placeholder = true;
            if let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    out.push_str(&enum_name.to_string());
                    continue;
                }
            }

            let mut inner = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                inner.push(next);
            }

            match inner.as_str() {
                "enum" => out.push_str(&enum_name.to_string()),
                "" => {
                    return Err(syn::Error::new(
                        enum_name.span(),
                        "empty placeholder `{}` is not allowed; use `{enum}`",
                    ))
                }
                "variant" => {
                    return Err(syn::Error::new(
                        enum_name.span(),
                        "`{variant}` placeholder is not supported for extractor names",
                    ))
                }
                _ => {
                    return Err(syn::Error::new(
                        enum_name.span(),
                        "unknown placeholder; expected `{enum}`",
                    ))
                }
            }
        } else {
            out.push(ch);
        }
    }

    Ok((out, has_placeholder))
}

fn format_selector_name(
    format: &str,
    enum_name: &Ident,
    variant_name: &Ident,
) -> syn::Result<(String, bool)> {
    // Support placeholder-based formatting in two styles:
    // 1) "Select{}{}" (positional): first `{}` is enum, second is variant
    // 2) "Select{enum}{variant}" (named)
    // Anything else is emitted verbatim.

    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut positional_count = 0;
    let mut has_placeholder = false;

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if let Some(&next) = chars.peek() {
                if next == '}' {
                    has_placeholder = true;
                    // positional placeholder
                    chars.next();
                    positional_count += 1;
                    match positional_count {
                        1 => out.push_str(&enum_name.to_string()),
                        2 => out.push_str(&variant_name.to_string()),
                        _ => {
                            return Err(syn::Error::new(
                                enum_name.span(),
                                "too many positional placeholders; expected at most 2",
                            ))
                        }
                    }
                    continue;
                }
            }

            // named placeholder
            let mut inner = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                inner.push(next);
            }

            match inner.as_str() {
                "enum" => {
                    has_placeholder = true;
                    out.push_str(&enum_name.to_string())
                }
                "variant" => {
                    has_placeholder = true;
                    out.push_str(&variant_name.to_string())
                }
                "" => {
                    return Err(syn::Error::new(
                        enum_name.span(),
                        "empty placeholder `{}` is not allowed; use `{enum}` or `{variant}`",
                    ))
                }
                _ => {
                    return Err(syn::Error::new(
                        enum_name.span(),
                        "unknown placeholder; expected `{enum}` or `{variant}`",
                    ))
                }
            }
        } else {
            out.push(ch);
        }
    }

    Ok((out, has_placeholder))
}

fn ident_from_string(span: proc_macro2::Span, s: &str) -> syn::Result<Ident> {
    syn::parse_str::<Ident>(s).map_err(|e| syn::Error::new(span, e))
}




/// Derive helper for `#[derive(ExtractFrom)]`.
///
/// Generates a zero-sized extractor type named `<EnumName>Extractor` (by default)
/// and implements the appropriate extraction traits following the decision tree in
/// `docs/four-crate-pattern-guide.md`.
///
/// # Customization
///
/// You may customize the generated names using `#[extract_from(...)]` attributes.
///
/// ## Extractor name
///
/// Override the extractor type name with:
///
/// ```rust,ignore
/// use split_by_discriminant_macros::ExtractFrom;
///
/// #[derive(ExtractFrom)]
/// #[extract_from(extractor = "MyExtractor")]
/// enum E { A(i32) }
/// ```
///
/// This will generate `struct MyExtractor;` instead of `struct EExtractor;`.
///
/// ## Selector name
///
/// For cases where `ExtractFrom` must generate selector types (multi-field
/// variants or ambiguous field types), you can customize selector names:
///
/// - Per-variant override:
///
/// ```rust,ignore
/// use split_by_discriminant_macros::ExtractFrom;
///
/// #[derive(ExtractFrom)]
/// enum E {
///     #[extract_from(selector = "MySelector")]
///     A(i32, String),
/// }
/// ```
///
/// - Global format string (uses `{}` or `{enum}/{variant}` placeholders):
///
/// ```rust,ignore
/// use split_by_discriminant_macros::ExtractFrom;
///
/// #[derive(ExtractFrom)]
/// #[extract_from(selector = "Custom{enum}{variant}")]
/// enum E { A(i32, String) }
/// ```
///
/// The default selector naming is `Select{Enum}{Variant}`.
///
/// # Notes
///
/// * The `extract_from` attribute is only enabled for this derive via
///   `#[proc_macro_derive(ExtractFrom, attributes(extract_from))]`.
/// * The attribute parser uses `syn::Attribute::parse_nested_meta` and
///   expects `key = "value"` string literals.
pub fn derive_extract_from(input: TokenStream) -> TokenStream {
    let di = match parse2::<DeriveInput>(input) {
        Ok(input) => match input.data {
            Data::Enum(_) => input,
            _ => return syn::Error::new_spanned(&input, "ExtractFrom can only be derived for enums").to_compile_error(),
        },
        Err(err) => return err.to_compile_error(),
    };

    let enum_name = &di.ident;
    let vis = &di.vis;
    let generics = &di.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let attrs = match parse_extract_from_attr(&di.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error(),
    };

    let e = match &di.data {
        Data::Enum(data) => data,
        _ => unreachable!(),
    };

    struct VariantInfo {
        name: Ident,
        is_named: bool,
        fields: Vec<FieldInfo>,
        selector_override: Option<String>,
    }

    struct FieldInfo {
        name: Option<Ident>,
        ty: Type,
    }

    fn is_reference(ty: &Type) -> bool {
        matches!(ty, Type::Reference(_))
    }

    let variants: Vec<VariantInfo> = match (|| -> syn::Result<_> {
        let mut out = Vec::new();
        for v in &e.variants {
            let selector_override = parse_extract_from_attr(&v.attrs)?.selector_format;

            match &v.fields {
                Fields::Unit => continue,
                Fields::Unnamed(fields) => {
                    out.push(VariantInfo {
                        name: v.ident.clone(),
                        is_named: false,
                        selector_override,
                        fields: fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| FieldInfo {
                                name: Some(format_ident!("v{}", i)),
                                ty: f.ty.clone(),
                            })
                            .collect(),
                    });
                }
                Fields::Named(fields) => {
                    out.push(VariantInfo {
                        name: v.ident.clone(),
                        is_named: true,
                        selector_override,
                        fields: fields
                            .named
                            .iter()
                            .map(|f| FieldInfo {
                                name: f.ident.clone(),
                                ty: f.ty.clone(),
                            })
                            .collect(),
                    });
                }
            }
        }
        Ok(out)
    })() {
        Ok(v) => v,
        Err(err) => return err.to_compile_error(),
    };

    if variants.is_empty() {
        return syn::Error::new_spanned(enum_name, "Cannot derive ExtractFrom for an enum with no fields").to_compile_error();
    }

    let any_multi_field = variants.iter().any(|v| v.fields.len() != 1);
    let any_ref_field = variants
        .iter()
        .flat_map(|v| v.fields.iter())
        .any(|f| is_reference(&f.ty));

    let mut seen = std::collections::HashSet::new();
    let mut has_duplicate_type = false;
    for v in &variants {
        if v.fields.len() != 1 {
            has_duplicate_type = true;
            break;
        }
        // Note: stringify type for a quick equality check (reasonable for derives).
        let field_ty = &v.fields[0].ty;
        let ty_str = quote!(#field_ty).to_string();
        if !seen.insert(ty_str) {
            has_duplicate_type = true;
            break;
        }
    }

    enum Strategy {
        Simple,
        Variant,
        Extract,
    }

    let strategy = if variants.len() == 1 && !any_ref_field && !any_multi_field {
        Strategy::Simple
    } else if !any_ref_field && !any_multi_field && !has_duplicate_type {
        Strategy::Variant
    } else {
        Strategy::Extract
    };

    let (extractor_name, emit_extractor_struct) = match &attrs.extractor {
        Some(name) => match format_extractor_name(name, enum_name)
            .and_then(|(s, formatted)| {
                let id = ident_from_string(enum_name.span(), &s)?;
                Ok((id, formatted))
            })
        {
            Ok((id, formatted)) => (id, formatted),
            Err(err) => return err.to_compile_error(),
        },
        None => (format_ident!("{}Extractor", enum_name), true),
    };

    let selector_info: Vec<(Ident, bool)> = match variants
        .iter()
        .map(|variant| {
            let (selector_str, is_generated) = if let Some(override_name) = &variant.selector_override {
                // Explicit selector name: assume shared and externally provided.
                (override_name.clone(), false)
            } else if let Some(format) = &attrs.selector_format {
                let (s, formatted) = format_selector_name(format, enum_name, &variant.name)?;
                (s, formatted)
            } else {
                // Default naming scheme creates a generated selector type.
                (format!("Select{}{}", enum_name, variant.name), true)
            };

            let selector_ident = ident_from_string(enum_name.span(), &selector_str)?;
            Ok((selector_ident, is_generated))
        })
        .collect::<syn::Result<Vec<(Ident, bool)>>>()
    {
        Ok(v) => v,
        Err(err) => return err.to_compile_error(),
    };

    let extractor_struct = if emit_extractor_struct {
        quote! { #vis struct #extractor_name; }
    } else {
        quote! {}
    };

    let impls = match strategy {
        Strategy::Simple => {
            let variant = &variants[0];
            let field_type = &variant.fields[0].ty;
            let field_bind = &variant.fields[0].name;
            let variant_ident = &variant.name;

            quote! {
                impl #impl_generics split_by_discriminant::SimpleExtractFrom<#enum_name #ty_generics> for #extractor_name #where_clause {
                    type Output = #field_type;

                    fn extract_from<'a>(&self, t: &'a mut #enum_name #ty_generics) -> Option<&'a mut Self::Output> {
                        if let #enum_name::#variant_ident(ref mut #field_bind) = *t {
                            Some(#field_bind)
                        } else {
                            None
                        }
                    }
                }
            }
        }
        Strategy::Variant => {
            let impls = variants.iter().map(|variant| {
                let field_type = &variant.fields[0].ty;
                let field_bind = &variant.fields[0].name;
                let variant_ident = &variant.name;

                quote! {
                    impl #impl_generics split_by_discriminant::VariantExtractFrom<#enum_name #ty_generics, #field_type> for #extractor_name #where_clause {
                        fn extract_from<'a>(&self, t: &'a mut #enum_name #ty_generics) -> Option<&'a mut #field_type> {
                            if let #enum_name::#variant_ident(ref mut #field_bind) = *t {
                                Some(#field_bind)
                            } else {
                                None
                            }
                        }
                    }
                }
            });
            quote! { #(#impls)* }
        }
        Strategy::Extract => {
            let selector_defs = selector_info
                .iter()
                .filter(|(_, is_generated)| *is_generated)
                .map(|(selector, _)| quote! { #vis struct #selector; });

            let impls = variants
                .iter()
                .zip(selector_info.iter().map(|(selector, _)| selector))
                .map(|(variant, selector)| {
                    let variant_ident = &variant.name;

                let field_binds: Vec<Ident> = variant
                    .fields
                    .iter()
                    .map(|f| f.name.clone().unwrap())
                    .collect();

                let output_type = if variant.fields.len() == 1 {
                    let ty = &variant.fields[0].ty;
                    quote! { &'a mut #ty }
                } else {
                    let tys = variant.fields.iter().map(|f| {
                        let ty = &f.ty;
                        quote! { &'a mut #ty }
                    });
                    quote! { ( #(#tys),* ) }
                };

                let output_expr = if field_binds.len() == 1 {
                    let b = &field_binds[0];
                    quote! { #b }
                } else {
                    quote! { ( #(#field_binds),* ) }
                };

                let pattern = if variant.is_named {
                    let field_bindings = variant.fields.iter().map(|f| {
                        let name = f.name.as_ref().unwrap();
                        quote! { #name: ref mut #name }
                    });
                    quote! { { #(#field_bindings),* } }
                } else {
                    let binds = field_binds.iter();
                    quote! { ( #(ref mut #binds),* ) }
                };

                quote! {
                    impl #impl_generics split_by_discriminant::ExtractFrom<#enum_name #ty_generics, #selector> for #extractor_name #where_clause {
                        type Output<'a> = #output_type where #enum_name #ty_generics: 'a;

                        fn extract_from<'a>(&self, t: &'a mut #enum_name #ty_generics) -> Option<Self::Output<'a>> {
                            if let #enum_name::#variant_ident #pattern = *t {
                                Some(#output_expr)
                            } else {
                                None
                            }
                        }
                    }
                }
            });

            quote! {
                #(#selector_defs)*
                #(#impls)*
            }
        }
    };

    quote! {
        #extractor_struct
        #impls
    }
}
