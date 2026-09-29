use slipstream_derive::Inspect;

#[test]
fn derive_struct_test() {
    #[derive(Inspect)]
    struct Test {
        #[inspect(category = "Some category", min = 0, max = 100)]
        pub field1: u32,
        #[inspect(category = "Some category", hidden)]
        field2: u64
    }
}