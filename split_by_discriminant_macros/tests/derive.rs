use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use split_by_discriminant_macros::ExtractFrom;
use std::mem::discriminant;

#[derive(Debug, PartialEq, ExtractFrom)]
enum E {
    A(i32),
    B(String),
    C,
}

#[test]
fn derive_extract_from_multiple_variants() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
    let mut ex = SplitWithExtractor::new(split, EExtractor);

    // SimpleExtractFrom is not generated (multiple variants), but VariantExtractFrom is.
    let ints: Vec<&mut i32> = ex.extract(a_disc).unwrap();
    assert_eq!(ints.len(), 2);

    let strs: Vec<&mut String> = ex.extract(b_disc).unwrap();
    assert_eq!(strs.len(), 1);
}

#[derive(Debug, PartialEq, ExtractFrom)]
enum Multi { A(i32, String), B(i32) }

#[test]
fn derive_extract_from_multi_field() {
    let mut data = [Multi::A(1, "x".into()), Multi::B(2)];
    let a_disc = discriminant(&Multi::A(0, "".into()));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, MultiExtractor);

    // Multi-field variant yields an ExtractFrom impl with a selector.
    let pairs: Vec<(&mut i32, &mut String)> = ex.extract_gat::<SelectMultiA>(a_disc).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(*pairs[0].0, 1);
}

#[derive(Debug, PartialEq, ExtractFrom)]
enum Single { A(i32) }

#[test]
fn derive_extract_from_single_variant() {
    let mut data = [Single::A(4), Single::A(5)];
    let a_disc = discriminant(&Single::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, SingleExtractor);

    // Should generate SimpleExtractFrom and allow extract_simple without annotation.
    let ints = ex.extract_simple(a_disc).unwrap();
    assert_eq!(ints, vec![&mut 4, &mut 5]);
}

struct CustomExtractor;

#[derive(Debug, PartialEq, ExtractFrom)]
#[extract_from(extractor = "CustomExtractor")]
enum CustomName { A(i32) }

#[test]
fn derive_extract_from_custom_extractor_name() {
    let mut data = [CustomName::A(1), CustomName::A(2)];
    let a_disc = discriminant(&CustomName::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, CustomExtractor);

    let ints = ex.extract_simple(a_disc).unwrap();
    assert_eq!(ints, vec![&mut 1, &mut 2]);
}

#[derive(Debug, PartialEq, ExtractFrom)]
#[extract_from(extractor = "Custom{}Extractor")]
enum Formatted { A(i32) }

#[test]
fn derive_extract_from_formatted_extractor_name() {
    let mut data = [Formatted::A(1), Formatted::A(2)];
    let a_disc = discriminant(&Formatted::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, CustomFormattedExtractor);

    let ints = ex.extract_simple(a_disc).unwrap();
    assert_eq!(ints, vec![&mut 1, &mut 2]);
}

#[derive(Debug, PartialEq, ExtractFrom)]
#[extract_from(extractor = "Custom{enum}Extractor")]
enum FormattedEnumPlaceholder { A(i32) }

#[test]
fn derive_extract_from_formatted_enum_extractor_name() {
    let mut data = [FormattedEnumPlaceholder::A(1), FormattedEnumPlaceholder::A(2)];
    let a_disc = discriminant(&FormattedEnumPlaceholder::A(0));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, CustomFormattedEnumPlaceholderExtractor);

    let ints = ex.extract_simple(a_disc).unwrap();
    assert_eq!(ints, vec![&mut 1, &mut 2]);
}

struct MySelector;

#[derive(Debug, PartialEq, ExtractFrom)]
enum PerVariantSelector {
    #[extract_from(selector = "MySelector")]
    A(i32, String),
}

#[test]
fn derive_extract_from_variant_selector_override() {
    let mut data = [PerVariantSelector::A(1, "x".into())];
    let a_disc = discriminant(&PerVariantSelector::A(0, "".into()));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, PerVariantSelectorExtractor);

    let pairs: Vec<(&mut i32, &mut String)> = ex.extract_gat::<MySelector>(a_disc).unwrap();
    assert_eq!(pairs.len(), 1);
}

#[derive(Debug, PartialEq, ExtractFrom)]
#[extract_from(selector = "Custom{enum}{variant}")]
enum GlobalSelectorFormat { A(i32, String) }

#[test]
fn derive_extract_from_global_selector_format() {
    let mut data = [GlobalSelectorFormat::A(1, "x".into())];
    let a_disc = discriminant(&GlobalSelectorFormat::A(0, "".into()));

    let split = split_by_discriminant(&mut data, &[a_disc]);
    let mut ex = SplitWithExtractor::new(split, GlobalSelectorFormatExtractor);

    let pairs: Vec<(&mut i32, &mut String)> = ex.extract_gat::<CustomGlobalSelectorFormatA>(a_disc).unwrap();
    assert_eq!(pairs.len(), 1);
}
