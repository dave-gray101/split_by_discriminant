// Fixture file for emulate_functionlike_macro_expansion("make_extractor", ...).
// NOT compiled as a test binary — opened as a File by tarpaulin.rs at runtime.

// Auto-naming: no extractor or fn_name → defaults derived from group name
make_extractor! { AutoGroup }

// Explicit extractor and fn_name
make_extractor! { NamedGroup, extractor = NamedGroupEx, fn_name = named_make }

// "function" as alias for fn_name
make_extractor! { FunctionAliasGroup, extractor = FunctionAliasEx, function = make_function_alias }

// trait_name: non-empty t_trait_bound and trait_docs branch
make_extractor! { TraitGroup, extractor = TraitGroupEx, fn_name = make_trait_group, trait_name = SomeTrait }

// ref_fn_name: emit the read-only companion function
make_extractor! { RefFnGroup, extractor = RefFnGroupEx, fn_name = make_ref_fn_mut, ref_fn_name = make_ref_fn_ref }

// "ref_function" as alias for ref_fn_name
make_extractor! { RefFuncAlias, extractor = RefFuncAliasEx, fn_name = make_rfa_mut, ref_function = make_rfa_ref }

// trait_name + ref_fn_name together (both non-empty branches simultaneously)
make_extractor! { FullGroup, extractor = FullGroupEx, fn_name = make_full_mut, ref_fn_name = make_full_ref, trait_name = FullTrait }

// Error: unknown key (parse error → compile_error TokenStream, no panic)
make_extractor! { ErrorGroup, unknown_key = BadValue }
