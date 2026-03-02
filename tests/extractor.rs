mod common;
use common::*;

use split_by_discriminant::{split_by_discriminant, SplitWithExtractor};
use std::mem::discriminant;

#[test]
fn split_with_extractor_and_extract() {
    let mut data = [E::A(1), E::B("hi".into()), E::A(2), E::C];
    let a_disc = discriminant(&E::A(0));
    let b_disc = discriminant(&E::B(String::new()));

    {
        let split = split_by_discriminant(&mut data, &[a_disc, b_disc]);
        let mut extractor = SplitWithExtractor::new(split, EExtractor);

        // raw group access still available on SplitWithExtractor directly
        assert_eq!(extractor.group(a_disc).unwrap().len(), 2);

        // ergonomic extraction via the extractor extractor
        let mut ints: Vec<&mut i32> = extractor.extract(a_disc).unwrap();
        assert_eq!(ints.len(), 2);
        *ints[0] = 10;

        let mut strings: Vec<&mut String> = extractor.extract(b_disc).unwrap();
        assert_eq!(strings.len(), 1);
        strings[0].push_str("!");
    }

    assert_eq!(data[0], E::A(10));
    assert_eq!(data[1], E::B("hi!".into()));
}
