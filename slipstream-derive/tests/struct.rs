use slipstream_derive::Inspect;

#[test]
fn derive_struct_test() {
    #[derive(Inspect)]
    #[inspect(label = "sldjklsdjs")]
    struct Test {
        #[inspect(category = "Some category")]
        pub field1: u32,
        #[inspect(rename = "field3", category = "Some category", hidden = false)]
        field2: u64,
    }
}
